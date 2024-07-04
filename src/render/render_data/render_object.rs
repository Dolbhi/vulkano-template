use std::sync::Arc;

use cgmath::{Matrix4, SquareMatrix};

use crate::{vulkano_objects::buffers::MeshBuffers, VertexFull};

use super::material::RenderSubmit;

#[derive(Debug, Clone)]
/// Data for standard rendering of a mesh
/// Type T is the additional data type of the object
pub struct RenderObject<T: Clone> {
    pub mesh: Arc<MeshBuffers<VertexFull>>,
    pub model: Matrix4<f32>,
    pub material: RenderSubmit<T>,
    pub data: T,
}

impl<T: Clone> RenderObject<T> {
    pub fn new(mesh: Arc<MeshBuffers<VertexFull>>, material: RenderSubmit<T>, data: T) -> Self {
        Self {
            mesh,
            model: Matrix4::identity(),
            material,
            data,
        }
    }

    /// Adds the render object's mesh and data to its material's render queue
    pub fn upload(&self) {
        self.material
            .lock()
            .unwrap()
            .push((self.mesh.clone(), self.model, self.data.clone()));
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
