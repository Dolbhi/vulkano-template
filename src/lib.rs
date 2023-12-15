pub mod app;
pub mod game_objects;
pub mod render;
pub mod shaders;
mod vertex_data;
pub mod vulkano_objects;

pub use vertex_data::{Vertex2d, Vertex3d, VertexFull};

use crate::{
    game_objects::transform::TransformCreateInfo,
    render::{mesh::from_obj, RenderObject},
};
use cgmath::Matrix4;
use game_objects::transform::{TransformID, TransformSystem};
use legion::World;
use render::{DrawSystem, MaterialID, Renderer};
use shaders::basic::vs::GPUObjectData;
use std::{iter::zip, path::Path, sync::Arc};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

fn init_render_objects(
    world: &mut World,
    transform_sys: &mut TransformSystem,
    renderer: &Renderer,
    draw_system: &mut DrawSystem<GPUObjectData, Matrix4<f32>>,
) -> TransformID {
    let resource_loader = renderer.get_resource_loader();
    let basic_id = 0;
    let phong_id = 1;
    let uv_id = 2;

    // Texture
    let le_texture = resource_loader.load_texture(Path::new("models/lost_empire-RGBA.png"));

    let ina_textures = [
        "models/ina/Hair_Base_Color.png",
        "models/ina/Cloth_Base_Color.png",
        "models/ina/Body_Base_Color.png",
        "models/ina/Head_Base_Color.png",
    ]
    .map(|p| resource_loader.load_texture(Path::new(p)));

    let linear_sampler = resource_loader.load_sampler(vulkano::image::sampler::Filter::Linear);

    // materials
    //  lost empire
    let le_mat_id = draw_system.add_material(
        basic_id,
        "lost_empire",
        Some(resource_loader.load_material_set(
            draw_system.get_pipeline(basic_id),
            le_texture.clone(),
            linear_sampler.clone(),
        )),
    );
    let le_lit_mat_id = draw_system.add_material(
        phong_id,
        "lost_empire_lit",
        Some(resource_loader.load_material_set(
            draw_system.get_pipeline(phong_id),
            le_texture,
            linear_sampler.clone(),
        )),
    );

    //  ina
    let ina_ids: Vec<_> = zip(["hair", "cloth", "body", "head"], ina_textures)
        .map(|(id, tex)| {
            draw_system.add_material(
                phong_id,
                id,
                Some(resource_loader.load_material_set(
                    draw_system.get_pipeline(phong_id),
                    tex,
                    linear_sampler.clone(),
                )),
            )
        })
        .collect();

    //  uv
    let uv_mat_id = draw_system.add_material(uv_id, "uv", None);

    // meshes
    //      suzanne
    let (vertices, indices) = from_obj(Path::new("models/suzanne.obj")).pop().unwrap();
    let suzanne_mesh = resource_loader.load_mesh(vertices, indices);

    //      square
    let vertices = vec![
        VertexFull {
            position: [-0.25, -0.25, 0.0],
            normal: [0.0, 0.0, 1.0],
            colour: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        },
        VertexFull {
            position: [0.25, -0.25, 0.0],
            normal: [0.0, 0.0, 1.0],
            colour: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
        },
        VertexFull {
            position: [-0.25, 0.25, 0.0],
            normal: [0.0, 0.0, 1.0],
            colour: [0.0, 1.0, 0.0],
            uv: [0.0, 1.0],
        },
        VertexFull {
            position: [0.25, 0.25, 0.0],
            normal: [0.0, 0.0, 1.0],
            colour: [0.0, 1.0, 0.0],
            uv: [1.0, 1.0],
        },
    ];
    let indices = vec![0, 1, 2, 2, 1, 3];
    let square = resource_loader.load_mesh(vertices, indices);

    //      lost empire
    let le_meshes: Vec<_> = from_obj(Path::new("models/lost_empire.obj"))
        .into_iter()
        .map(|(vertices, indices)| resource_loader.load_mesh(vertices, indices))
        .collect();

    //      ina
    let ina_meshes: Vec<_> = from_obj(Path::new("models/ina/ReadyToRigINA.obj"))
        .into_iter()
        .skip(2)
        .map(|(vertices, indices)| resource_loader.load_mesh(vertices, indices))
        .collect();

    println!("[Rendering Data]");
    println!("Lost empire mesh count: {}", le_meshes.len());
    println!("Ina mesh count: {}", ina_meshes.len());

    // objects
    //  Suzanne
    let suzanne_obj = Arc::new(RenderObject::new(suzanne_mesh, uv_mat_id.clone()));
    let suzanne = transform_sys.next().unwrap();
    world.push((suzanne, suzanne_obj));

    //  Squares
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        let square_obj = Arc::new(RenderObject::new(square.clone(), uv_mat_id.clone()));
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            translation: [x, y, z].into(),
            ..Default::default()
        });

        world.push((transform_id, square_obj));
    }

    //  Ina
    let ina_transform = transform_sys.add_transform(TransformCreateInfo {
        translation: [0.0, 5.0, -1.0].into(),
        ..Default::default()
    });
    // world.push((ina_transform));
    for (mesh, mat_id) in zip(ina_meshes, ina_ids.clone()) {
        let obj = Arc::new(RenderObject::new(mesh, mat_id));
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            parent: Some(ina_transform),
            ..Default::default()
        });

        world.push((transform_id, obj));
    }

    //  lost empires
    let le_transform = transform_sys.add_transform(TransformCreateInfo::default());
    for mesh in le_meshes {
        let le_obj = Arc::new(RenderObject::new(mesh, le_mat_id.clone()));
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            parent: Some(le_transform),
            ..Default::default()
        });

        let mat_swapper = MaterialSwapper::new([
            le_mat_id.clone(),
            le_lit_mat_id.clone(),
            uv_mat_id.clone(),
            "cloth".into(),
        ]);

        world.push((transform_id, le_obj, mat_swapper));
    }

    suzanne
}

pub struct MaterialSwapper {
    materials: Vec<MaterialID>,
    curent_index: usize,
}
impl MaterialSwapper {
    fn new(materials: impl IntoIterator<Item = impl Into<MaterialID>>) -> Self {
        let materials = materials.into_iter().map(|m| m.into()).collect();
        Self {
            materials,
            curent_index: 0,
        }
    }

    pub fn swap_material(&mut self) -> MaterialID {
        self.curent_index = (self.curent_index + 1) % self.materials.len();
        self.materials[self.curent_index].clone()
    }
}
