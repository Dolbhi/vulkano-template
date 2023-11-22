use std::sync::Arc;

use cgmath::{Matrix4, Rad, SquareMatrix};

use crate::{vulkano_objects::buffers::Buffers, VertexFull};

use super::material::Material;

pub struct RenderObject {
    pub mesh: Arc<Buffers<VertexFull>>,
    pub material: Arc<Material>,
    transform: Matrix4<f32>,
}

impl RenderObject {
    pub fn new(mesh: Arc<Buffers<VertexFull>>, material: Arc<Material>) -> Self {
        Self {
            mesh,
            material,
            // uniforms,
            transform: Matrix4::identity(),
        }
    }

    pub fn get_transform_matrix(&self) -> Matrix4<f32> {
        self.transform
    }

    pub fn update_transform(&mut self, position: [f32; 3], rotation: Rad<f32>) {
        let rotation = Matrix4::from_axis_angle([0., 1., 0.].into(), rotation);
        let translation = Matrix4::from_translation(position.into());

        self.transform = translation * rotation;
    }

    // pub fn update_transform_axis(
    //     &mut self,
    //     position: [f32; 3],
    //     rotation: Rad<f32>,
    //     axis: [f32; 3],
    // ) {
    //     let rotation = Matrix4::from_axis_angle(axis.into(), rotation);
    //     let translation = Matrix4::from_translation(position.into());

    //     self.transform = translation * rotation;
    // }
}
