use std::{iter, sync::Arc};

use anyhow::Result;
use camera::{Camera, CameraController};
use context::WgpuContext;
use sdl2::{event, video::Window};
use instance::{Instance, InstanceManager, InstanceRaw};
use draw::DrawModel;

mod model;
mod resources;
mod texture;
mod camera;
pub mod instance;
mod draw;
mod context;

use model::{texture_to_model, Vertex};

const NUM_INSTANCES_PER_ROW: u32 = 10;

pub trait InstanceAble {
    fn to_instance(&self) -> Instance;
    fn to_raw(&self) -> InstanceRaw {
        self.to_instance().to_raw()
    }
}


#[derive(Debug, Clone, Copy)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

pub struct WgpuEngine<'w> {
    context: WgpuContext<'w>,
    render_pipeline: wgpu::RenderPipeline,
    camera: Camera,
    depth_texture: texture::Texture,
}

impl<'w> WgpuEngine<'w> {
    pub async fn new(window: &'w Window) -> Result<WgpuEngine<'w>> {
        let context = WgpuContext::new(window).await?;

        let texture_bind_group_layout =
            context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let camera_bind_group_layout =
        context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let camera = Camera::new(
            camera::LookAt::new((0.0, 3.0, 10.0).into(), (0.0, 0.0, 0.0).into(), (0.0, 1.0, 0.0).into()),
            Box::new(camera::PerspectiveProjection::new(context.size.width as f32 / context.size.height as f32, 45.0, 0.1, 100.0)
            ),
            &context.device,
            &camera_bind_group_layout,
        );

        log::warn!("Load model");
        let obj_model = Arc::new(
            texture_to_model(
                resources::load_texture("cube-diffuse.jpg", &context.device, &context.queue).await?,
                &texture_bind_group_layout,
                &context.device,
                "box",
            ));
        let mut instance_manager = instance::InstanceManager::new(&context.device, obj_model.clone()); 

        const SPACE_BETWEEN: f32 = 3.0;
        for i in 0..NUM_INSTANCES_PER_ROW {
            for j in 0..NUM_INSTANCES_PER_ROW {
                let x = SPACE_BETWEEN * (i as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (j as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position = ultraviolet::Vec3 { x, y: 0.0, z };

                let rotation = if position.mag() == 0.0 {
                    ultraviolet::Rotor3::identity()
                } else {
                    ultraviolet::Rotor3::from_rotation_between(
                        ultraviolet::Vec3::unit_z(),
                        position.normalized(),
                    )
                };

                let instance = Instance { position, rotation, id: (i * NUM_INSTANCES_PER_ROW + j) as u128 , scale: 1.0};
                instance_manager.add_instance(&context.device, &context.queue, instance);
            }
        }


        let shader = context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shader.wgsl").into()),
        });

        let depth_texture =
            texture::Texture::create_depth_texture(&context.device, &context.config, "depth_texture");

        let render_pipeline_layout =
            context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[model::ModelVertex::desc(), InstanceRaw::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });
        context.surface.configure(&context.device, &context.config);
        Ok(Self {
            context,
            render_pipeline,
            camera,
            depth_texture,
        })
    }

    pub fn window(&self) -> &Window {
        &self.context.window
    }

    pub fn resize(&mut self, new_size: WindowSize) {
        if new_size.width > 0 && new_size.height > 0 {
            self.context.config.width = new_size.width;
            self.context.config.height = new_size.height;
            self.context.size = new_size;
            self.camera.projection.resize(self.context.config.width as f32, self.context.config.height as f32);
            self.context.surface.configure(&self.context.device, &self.context.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.context.device, &self.context.config, "depth_texture");
        }
    }

    pub fn update(&mut self) -> Result<()> {
        log::info!("{:?}", self.camera);
        self.camera.update(&self.context.queue);
        Ok(())
    }

    pub fn render(&mut self, to_draw: &[InstanceManager]) -> Result<()> {
        let output = self.context.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .context.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            for i in to_draw {
                render_pass.draw_instances(i, &self.camera.bind_group());
            }
        }

        self.context.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
