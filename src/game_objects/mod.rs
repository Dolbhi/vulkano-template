mod camera;
mod game_world;
pub mod light;
pub mod transform;
pub mod utility;

use std::sync::Arc;

pub use camera::Camera;
pub use game_world::{GameWorld, Inputs};

use cgmath::{Rad, Vector3};

use crate::{
    render::{RenderObject, RenderSubmit},
    vulkano_objects::buffers::MeshBuffers,
    VertexFull,
};

use self::transform::{TransformCreateInfo, TransformID};

#[derive(Debug)]
pub struct NameComponent(pub String);
pub struct Rotate(pub Vector3<f32>, pub Rad<f32>);
pub struct DisabledLERP;

#[derive(Clone)]
pub struct MaterialSwapper<T: Clone> {
    materials: Vec<RenderSubmit<T>>,
    curent_index: usize,
}

pub struct WorldLoader<'a>(
    pub &'a mut legion::World,
    pub &'a mut transform::TransformSystem,
);

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

impl<'a> WorldLoader<'a> {
    pub fn quick_ro(
        &mut self,
        transform: impl Into<TransformCreateInfo>,
        mesh: Arc<MeshBuffers<VertexFull>>,
        material: RenderSubmit<()>,
    ) -> (TransformID, legion::Entity) {
        self.add_1_comp(transform, RenderObject::new(mesh, material, ()))
    }

    pub fn add_1_comp<T>(
        &mut self,
        transform: impl Into<TransformCreateInfo>,
        comp: T,
    ) -> (TransformID, legion::Entity)
    where
        T: legion::storage::Component,
    {
        let id = self.1.add_transform(transform.into());
        (id, self.0.push((id, comp)))
    }

    pub fn add_2_comp<T1, T2>(
        &mut self,
        transform: impl Into<TransformCreateInfo>,
        comp_1: T1,
        comp_2: T2,
    ) -> (TransformID, legion::Entity)
    where
        T1: legion::storage::Component,
        T2: legion::storage::Component,
    {
        let id = self.1.add_transform(transform.into());
        (id, self.0.push((id, comp_1, comp_2)))
    }

    pub fn add_3_comp<T1, T2, T3>(
        &mut self,
        transform: impl Into<TransformCreateInfo>,
        comp_1: T1,
        comp_2: T2,
        comp_3: T3,
    ) -> (TransformID, legion::Entity)
    where
        T1: legion::storage::Component,
        T2: legion::storage::Component,
        T3: legion::storage::Component,
    {
        let id = self.1.add_transform(transform.into());
        (id, self.0.push((id, comp_1, comp_2, comp_3)))
    }
}
