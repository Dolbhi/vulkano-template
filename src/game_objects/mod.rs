mod camera;
mod game_world;
pub mod light;
pub mod transform;
pub mod utility;

pub use camera::Camera;
pub use game_world::{GameWorld, Inputs};

use cgmath::{Matrix4, Rad, Vector3};

use crate::render::RenderSubmit;

#[derive(Debug)]
pub struct NameComponent(pub String);
pub struct Rotate(pub Vector3<f32>, pub Rad<f32>);
pub struct DisabledLERP;

pub struct MaterialSwapper {
    materials: Vec<RenderSubmit<Matrix4<f32>>>,
    curent_index: usize,
}
impl MaterialSwapper {
    pub fn new(materials: impl IntoIterator<Item = RenderSubmit<Matrix4<f32>>>) -> Self {
        let materials = materials.into_iter().collect();
        Self {
            materials,
            curent_index: 0,
        }
    }

    pub fn swap_material(&mut self) -> RenderSubmit<Matrix4<f32>> {
        self.curent_index = (self.curent_index + 1) % self.materials.len();
        self.materials[self.curent_index].clone()
    }
}
