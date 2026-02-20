use std::sync::Arc;

use cgmath::{Matrix4, SquareMatrix};

use crate::{
    game_objects::transform::{TransformID, TransformSystem},
    vulkano_objects::buffers::MeshBuffers,
    VertexFull,
};

use super::material::RenderSubmit;

#[derive(Debug, Clone)]
/// Data for standard rendering of a mesh
/// Type T is the additional data type of the object
pub struct RenderObject<T: Clone> {
    pub mesh: Arc<MeshBuffers<VertexFull>>,
    // pub model: Matrix4<f32>,
    pub material: RenderSubmit<T>,
    pub data: T,
    pub lerp: bool,
}

impl<T: Clone> RenderObject<T> {
    pub fn new(mesh: Arc<MeshBuffers<VertexFull>>, material: RenderSubmit<T>, data: T) -> Self {
        Self {
            mesh,
            // model: Matrix4::identity(),
            material,
            data,
            lerp: true,
        }
    }

    /// Get transform matrix
    ///
    /// Warning: Contains unhandled unwrap from accessing transform
    pub fn update_and_upload(
        &mut self,
        transform_id: &TransformID,
        transforms: &mut TransformSystem,
    ) {
        let transfrom_matrix = if self.lerp {
            transforms.get_lerp_model(transform_id)
        } else {
            transforms.get_global_model(transform_id)
        };
        // println!("Obj {:?}: {:?}", transform_id, obj);
        // self.model = transfrom_matrix.unwrap();
        self.upload(transfrom_matrix.unwrap());
    }

    /// Adds the render object's mesh and data to its material's render queue (`RenderSubmit`)
    pub fn upload(&self, transform_matrix: Matrix4<f32>) {
        self.material.lock().unwrap().push((
            self.mesh.clone(),
            transform_matrix,
            self.data.clone(),
        ));
    }
}

impl<T: Clone> From<(Arc<MeshBuffers<VertexFull>>, RenderSubmit<T>, T)> for RenderObject<T> {
    fn from(value: (Arc<MeshBuffers<VertexFull>>, RenderSubmit<T>, T)) -> Self {
        Self::new(value.0, value.1, value.2)
    }
}

impl<T: Default + Clone> RenderObject<T> {
    /// Create render object with data set to its default value
    pub fn new_default_data(mesh: Arc<MeshBuffers<VertexFull>>, material: RenderSubmit<T>) -> Self {
        Self::new(mesh, material, T::default())
    }
}
