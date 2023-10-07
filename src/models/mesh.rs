use std::path::Path;

use vulkano::buffer::BufferContents;

use crate::models::Model;
use crate::VertexFull;

// use tobj::load_obj;

pub struct Mesh {
    vertices: Vec<VertexFull>,
    indices: Vec<u32>,
    // buffer: Buffers,
}

impl Mesh {
    pub fn from_obj(file_name: &Path) -> Self {
        let (models, _) = tobj::load_obj(file_name, &tobj::GPU_LOAD_OPTIONS).unwrap();
        let mesh = &models[0].mesh;

        // mesh.texcoords
        // mesh.material_id

        assert!(mesh.positions.len() % 3 == 0);

        let mut vertices = Vec::new();
        let length = mesh.positions.len() / 3;
        vertices.reserve_exact(length);

        // unflattern vertex data
        for i in 0..length {
            let first = i * 3;
            let position = [
                mesh.positions[first] / 3.,
                mesh.positions[first + 1] / 3.,
                mesh.positions[first + 2] / 3.,
            ];
            let normal = [
                mesh.normals[first],
                mesh.normals[first + 1],
                mesh.normals[first + 2],
            ];
            // let colour = [1., 0., 0.];
            //     mesh.vertex_color[first],
            //     mesh.vertex_color[first + 1],
            //     mesh.vertex_color[first + 2],
            // ];

            vertices.push(VertexFull {
                position,
                normal: normal.clone(),
                colour: normal,
            })
        }

        Mesh {
            vertices,
            indices: mesh.indices.clone(),
        }
    }

    pub fn from_model<U: BufferContents, M: Model<VertexFull, U>>() -> Self {
        Mesh {
            vertices: M::get_vertices(),
            indices: M::get_indices(),
        }
    }

    pub fn get_vertices(&self) -> Vec<VertexFull> {
        return self.vertices.clone();
    }

    pub fn get_indicies(&self) -> Vec<u32> {
        return self.indices.clone();
    }
}
