use std::{collections::HashMap, hash::Hash, sync::Arc};

use anyhow::{anyhow, Ok, Result};

use super::model::Model;

#[derive(Debug, Clone, Copy)]
pub struct Instance {
    pub id: u128,
    pub position: ultraviolet::Vec3,
    pub rotation: ultraviolet::Rotor3,
    pub scale: f32,
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (
                ultraviolet::Mat4::from_translation(self.position)
                * ultraviolet::Mat4::from_angle_plane(self.rotation.s, self.rotation.bv)
                * ultraviolet::Mat4::from_scale(self.scale)
            ).into(),
        }
    }
}


pub trait InstanceAble {
    fn to_instance(&self) -> Instance;
    fn to_raw(&self) -> InstanceRaw {
        self.to_instance().to_raw()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    #[allow(dead_code)]
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub const SIZE: u64 = size_of::<InstanceRaw>() as u64;

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct InstanceManager {
    pub model: Arc<Model>,
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
    id_to_index: HashMap<u128, usize>,
    remove_idxs: Vec<usize>,
}

impl InstanceManager {
    pub fn new(device: &wgpu::Device, model: Arc<Model>) -> Self {
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            model,
            instances: Vec::new(),
            instance_buffer,
            id_to_index: HashMap::new(),
            remove_idxs: Vec::new(),
        }
    }

    pub fn add_instance(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, instance: Instance) {
        if self.remove_idxs.len() > 0 {
            let index = self.remove_idxs.pop().unwrap();
            self.id_to_index.insert(instance.id, index);
            self.instances[index] = instance;
            self.update_instance(queue, instance).unwrap();
            return;
        }

        let raw_size = InstanceRaw::SIZE * (self.instances.len() + 1) as u64;
        let mut buffer_size = self.instance_buffer.size();
        if raw_size > buffer_size { 
            while raw_size > buffer_size { buffer_size *= 2; }
            self.instance_buffer.destroy();
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let raws = self.instances.iter().map(|i| i.to_raw()).collect::<Vec<InstanceRaw>>();
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&raws));
        }
        let raw = instance.to_raw();
        queue.write_buffer(&self.instance_buffer, raw_size - InstanceRaw::SIZE, bytemuck::cast_slice(&[raw]));
        self.id_to_index.insert(instance.id, self.instances.len());
        self.instances.push(instance);
    }

    pub fn update_instance(&mut self, queue: &wgpu::Queue, instance: Instance) -> Result<()> {
        if let Some(index) = self.id_to_index.get(&instance.id) {
            let raw = instance.to_raw();
            queue.write_buffer(&self.instance_buffer, (*index as u64) * size_of::<InstanceRaw>() as u64, bytemuck::cast_slice(&[raw]));
            self.instances[*index] = instance;
            Ok(())
        } else {
            Err(anyhow!("Instance not found"))
        }
    }

    pub fn remove_instance(&mut self, instance_id: u128) -> Result<Instance> {
        if let Some(index) = self.id_to_index.remove(&instance_id) {
            let instance = self.instances[index];
            self.remove_idxs.push(index);
            Ok(instance)
        } else {
            Err(anyhow!("Instance not found"))
        }

    }
}