use ultraviolet::{Rotor3, Vec2, Vec3};

use crate::wgpu_engine::{instance::Instance, InstanceAble};

#[derive(Debug, Clone, Copy)]
pub struct EulerRotation {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

impl EulerRotation {
    pub fn new(yaw: f32, pitch: f32, roll: f32) -> Self {
        Self { yaw, pitch, roll }
    }
    pub fn rotor3(&self) -> Rotor3 {
        Rotor3::from_euler_angles(self.yaw, self.pitch, self.roll)
    }
}

impl Default for EulerRotation {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

pub struct Transform {
    pub position: Vec3,
    pub euler_rotation: EulerRotation,
    pub scale: f32,
    pub id: u128,
}

impl Transform {
    pub fn new(position: Vec3, euler_rotation: EulerRotation, scale: f32, id: u128) -> Self {
        Self {
            position,
            euler_rotation,
            scale,
            id,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::zero(),
            euler_rotation: EulerRotation::default(),
            scale: 1.0,
            id: 0,
        }
    }
}

impl InstanceAble for Transform {
    fn to_instance(&self) -> Instance {
        Instance {
            position: self.position,
            rotation: self.euler_rotation.rotor3(),
            scale: self.scale,
            id: self.id,
        }
    }
}

pub struct Transform2d {
    pub position: Vec2,
    pub scale: f32,
    pub rotation: f32,
    pub id: u128,
}

impl Transform2d {
    pub fn new(position: Vec2, scale: f32, rotation: f32, id: u128) -> Self {
        Self {
            position,
            scale,
            rotation,
            id,
        }
    }
}

impl Default for Transform2d {
    fn default() -> Self {
        Self {
            position: Vec2::zero(),
            scale: 1.0,
            rotation: 0.0,
            id: 0,
        }
    }
}

impl InstanceAble for Transform2d {
    fn to_instance(&self) -> Instance {
        Instance {
            position: Vec3::new(self.position.x, self.position.y, 0.0),
            rotation: Rotor3::from_euler_angles(0.0, 0.0, self.rotation),
            scale: self.scale,
            id: self.id,
        }
    }
}