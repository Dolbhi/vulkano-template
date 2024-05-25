mod camera;
mod game_world;
pub mod light;
pub mod transform;
pub mod utility;

pub use camera::Camera;
pub use game_world::{GameWorld, Inputs};

use cgmath::{Rad, Vector3};

use crate::render::RenderSubmit;

#[derive(Debug)]
pub struct NameComponent(pub String);
pub struct Rotate(pub Vector3<f32>, pub Rad<f32>);
pub struct DisabledLERP;

pub struct MaterialSwapper<T: Clone> {
    materials: Vec<RenderSubmit<T>>,
    curent_index: usize,
}
impl<T: Clone> MaterialSwapper<T> {
    pub fn new(materials: impl IntoIterator<Item = RenderSubmit<T>>) -> Self {
        let materials = materials.into_iter().collect();
        Self {
            materials,
            curent_index: 0,
        }
    }

    pub fn swap_material(&mut self) -> RenderSubmit<T> {
        self.curent_index = (self.curent_index + 1) % self.materials.len();
        self.materials[self.curent_index].clone()
    }
}
