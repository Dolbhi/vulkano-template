use std::sync::Arc;

use cgmath::{Matrix4, Rad, SquareMatrix};

use crate::{vulkano_objects::buffers::Buffers, VertexFull};

use super::material::RenderSubmit;

#[derive(Debug)]
/// Data for standard rendering of a mesh
/// Type T is the additional data type of the object (usually a transform matrix)
pub struct RenderObject<T: Clone> {
    pub mesh: Arc<Buffers<VertexFull>>,
    pub material: RenderSubmit,
    pub data: T,
}

// impl<T: Clone> RenderObject<T> {
//     pub fn get_data(&self) -> T {
//         self.data.clone()
//     }
// }

impl RenderObject<Matrix4<f32>> {
    pub fn new(mesh: Arc<Buffers<VertexFull>>, material: RenderSubmit) -> Self {
        Self {
            mesh,
            material,
            // uniforms,
            data: Matrix4::identity(),
        }
    }

    pub fn set_matrix(&mut self, matrix: Matrix4<f32>) {
        self.data = matrix;
    }

    pub fn update_transform(&mut self, position: [f32; 3], rotation: Rad<f32>) {
        let rotation = Matrix4::from_axis_angle([0., 1., 0.].into(), rotation);
        let translation = Matrix4::from_translation(position.into());

        self.data = translation * rotation;
    }

    /// Adds the render object's mesh and data to its material's render queue
    pub fn upload(&self) {
        self.material
            .lock()
            .unwrap()
            .push((self.mesh.clone(), self.data.clone()));
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
