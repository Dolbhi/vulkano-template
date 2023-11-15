use std::path::Path;

use crate::VertexFull;

// use tobj::load_obj;

pub struct Mesh(pub Vec<VertexFull>, pub Vec<u32>);

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

        Mesh(vertices, mesh.indices.clone())
    }
}
