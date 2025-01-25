use std::fmt::Debug;

use sdl2::{event, keyboard::Keycode};
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct LookAt {
    pub eye: ultraviolet::Vec3,
    pub target: ultraviolet::Vec3,
    pub up: ultraviolet::Vec3,
}

impl LookAt {
    pub fn new(eye: ultraviolet::Vec3, target: ultraviolet::Vec3, up: ultraviolet::Vec3) -> Self {
        Self { eye, target, up }
    }
    fn view_mat(&self) -> ultraviolet::Mat4 {
        ultraviolet::Mat4::look_at(self.eye, self.target, self.up)
    }

    fn replace(&mut self, pos: ultraviolet::Vec3) {
        self.eye = pos;
    }

    fn go_forward(&mut self, speed: f32) {
        let forward = self.target - self.eye;
        let forward_mag = forward.mag();
        let forward = forward.normalized();
        if forward_mag > speed {
            self.eye += forward * speed;
        }
        self.eye += forward * speed;
    }

    fn rotate_eye(&mut self, rot: ultraviolet::Rotor3) {
        let forward = self.target - self.eye;
        self.eye = self.target - forward.rotated_by(rot) ;
    }

    fn rotate_target(&mut self, rot: ultraviolet::Rotor3) {
        let forward = self.target - self.eye;
        self.target = forward.rotated_by(rot) + self.eye;
    }
}

#[derive(Debug)]
pub struct Camera {
    view: LookAt,
    pub projection: Box<dyn Projection>,
    bind_group: wgpu::BindGroup,
    buffer: wgpu::Buffer,
    uniform: CameraUniform,
}

impl Camera {
    pub fn new(view: LookAt, projection: Box<dyn Projection>, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        
        let camera_uniform = CameraUniform::new();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        

        Self {
            view,
            projection,
            bind_group: camera_bind_group,
            buffer: camera_buffer,
            uniform: camera_uniform,
        }
    }
    fn build_view_proj_matrix(&self) -> ultraviolet::Mat4 {
        let view = self.view.view_mat();
        let proj = self.projection.proj_matrix();
        proj * view
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_view_proj(self.build_view_proj_matrix());
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

pub trait Projection: Debug {
    fn proj_matrix(&self) -> ultraviolet::Mat4;
    fn resize(&mut self, width: f32, height: f32);
}

#[derive(Debug)]
pub struct PerspectiveProjection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl PerspectiveProjection {
    pub fn new(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect,
            fovy: fovy.to_radians(),
            znear,
            zfar,
        }
    }
}

impl Projection for PerspectiveProjection {
    fn proj_matrix(&self) -> ultraviolet::Mat4 {
        ultraviolet::projection::perspective_wgpu_dx(
            self.fovy,
            self.aspect,
            self.znear,
            self.zfar,
        )
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
    }
}

#[derive(Debug)]
pub struct OrthographicProjection {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    znear: f32,
    zfar: f32,
}

impl OrthographicProjection {
    pub fn new(size: super::WindowSize, znear: f32, zfar: f32) -> Self {
        Self {
            left: 0.0,
            right: size.width as f32,
            bottom: 0.0,
            top: size.height as f32,
            znear,
            zfar,
        }
    }
}

impl Projection for OrthographicProjection {
    fn proj_matrix(&self) -> ultraviolet::Mat4 {
        ultraviolet::projection::orthographic_wgpu_dx(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.znear,
            self.zfar,
        )
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.right = width;
        self.top = height;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: ultraviolet::Mat4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, matrix: ultraviolet::Mat4) {
        self.view_proj = matrix.into();
    }
}

pub struct CameraController {
    speed: f32,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_events(&mut self, event: &event::Event) -> bool {
        match event {
            event::Event::KeyDown { keycode , .. } => {
                match keycode {
                    Some(Keycode::Space) => {
                        self.is_up_pressed = true;
                        true
                    }
                    Some(Keycode::LShift) => {
                        self.is_down_pressed = true;
                        true
                    }
                    Some(Keycode::W) | Some(Keycode::Up) => {
                        self.is_forward_pressed = true;
                        true
                    }
                    Some(Keycode::A) | Some(Keycode::Left) => {
                        self.is_left_pressed = true;
                        true
                    }
                    Some(Keycode::S) | Some(Keycode::Down) => {
                        self.is_backward_pressed = true;
                        true
                    }
                    Some(Keycode::D) | Some(Keycode::Right) => {
                        self.is_right_pressed = true;
                        true
                    }
                    _ => false,
                }
            }
            event::Event::KeyUp { keycode, ..} => {
                match keycode {
                    Some(Keycode::Space) => {
                        self.is_up_pressed = false;
                        true
                    }
                    Some(Keycode::LShift) => {
                        self.is_down_pressed = false;
                        true
                    }
                    Some(Keycode::W) | Some(Keycode::Up) => {
                        self.is_forward_pressed = false;
                        true
                    }
                    Some(Keycode::A) | Some(Keycode::Left) => {
                        self.is_left_pressed = false;
                        true
                    }
                    Some(Keycode::S) | Some(Keycode::Down) => {
                        self.is_backward_pressed = false;
                        true
                    }
                    Some(Keycode::D) | Some(Keycode::Right) => {
                        self.is_right_pressed = false;
                        true
                    },
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {

        // Prevents glitching when camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed{
            camera.view.go_forward(self.speed);
        }
        if self.is_backward_pressed {
            camera.view.go_forward(-self.speed);
        }
        if self.is_right_pressed {
            // Rescale the distance between the target and eye so
            // that it doesn't change. The eye therefore still
            // lies on the circle made by the target and eye.
            camera.view.rotate_eye(ultraviolet::Rotor3::from_rotation_xz(self.speed));
        }
        if self.is_left_pressed {
            camera.view.rotate_eye(ultraviolet::Rotor3::from_rotation_xz(-self.speed));
        }
    }
}
