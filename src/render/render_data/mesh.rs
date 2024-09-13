use std::path::Path;

use crate::VertexFull;

// use tobj::load_obj;

pub fn from_obj(file_name: &Path) -> Vec<(Vec<VertexFull>, Vec<u32>)> {
    let (models, _) = tobj::load_obj(file_name, &tobj::GPU_LOAD_OPTIONS).unwrap();
    models
        .into_iter()
        .map(|model| {
            let mesh = model.mesh;

            // mesh.texcoords
            // mesh.material_id

            assert!(mesh.positions.len() % 3 == 0);

            let length = mesh.positions.len() / 3;
            let mut vertices = Vec::with_capacity(length);

            // unflattern vertex data
            for i in 0..length {
                let first = i * 3;
                let position = [
                    mesh.positions[first],
                    mesh.positions[first + 1],
                    mesh.positions[first + 2],
                ];
                let normal = [
                    mesh.normals[first],
                    mesh.normals[first + 1],
                    mesh.normals[first + 2],
                ];
                let colour = [1., 1., 1.];
                //     mesh.vertex_color[first],
                //     mesh.vertex_color[first + 1],
                //     mesh.vertex_color[first + 2],
                // ];

                let uv = [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]];

                vertices.push(VertexFull {
                    position,
                    normal,
                    colour,
                    uv,
                })
            }

            (vertices, mesh.indices)
        })
        .collect()
}

// pub fn merge_meshes(meshes: &mut Vec<Mesh>) -> Self {
//     let mut vertices = Vec::new();
//     let mut indices = Vec::new();

//     for Mesh(v, i) in meshes {
//         vertices.append(v);
//         indices.append(i);
//     }

//     Mesh(vertices, indices)
// }
