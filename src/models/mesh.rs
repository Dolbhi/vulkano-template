use vulkano::buffer::BufferContents;

use crate::models::Model;
use crate::VertexFull;

pub struct Mesh {
    vertices: Vec<VertexFull>,
    indicies: Vec<u16>,
}

impl Mesh {
    pub fn from_model<U: BufferContents, M: Model<VertexFull, U>>() -> Self {
        Mesh {
            vertices: M::get_vertices(),
            indicies: M::get_indices(),
        }
    }

    pub fn get_vertices(&self) -> Vec<VertexFull> {
        return self.vertices.clone();
    }

    pub fn get_indicies(&self) -> Vec<u16> {
        return self.indicies.clone();
    }
}
