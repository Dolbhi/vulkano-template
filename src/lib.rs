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

use game_objects::transform::{TransformID, TransformSystem};
use legion::World;
use render::{Context, DrawSystem, RenderSubmit};
use std::{iter::zip, path::Path};

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
    context: &Context,
    draw_system: &mut DrawSystem,
) -> TransformID {
    let resource_loader = context.get_resource_loader();

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
    let (basic_pipeline, uv_pipeline) = if let [a, b] = &mut draw_system.pipelines[0..2] {
        (a, b)
    } else {
        panic!("Draw system somehow does not have 2 pipelines")
    };
    let le_mat = basic_pipeline.add_material(Some(resource_loader.load_material_set(
        basic_pipeline,
        le_texture.clone(),
        linear_sampler.clone(),
    )));
    let le_uv_mat = uv_pipeline.add_material(None);

    //  ina
    let ina_mats: Vec<_> = zip(["hair", "cloth", "body", "head"], ina_textures)
        .map(|(_, tex)| {
            basic_pipeline.add_material(Some(resource_loader.load_material_set(
                basic_pipeline,
                tex,
                linear_sampler.clone(),
            )))
        })
        .collect();

    //  uv
    let uv_mat = uv_pipeline.add_material(None);

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
    let suzanne_obj = RenderObject::new(suzanne_mesh, uv_mat.clone());
    let suzanne = transform_sys.next().unwrap();
    world.push((suzanne, suzanne_obj));

    //  Squares
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        let square_obj = RenderObject::new(square.clone(), uv_mat.clone());
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
    for (mesh, mat) in zip(ina_meshes, ina_mats.clone()) {
        let obj = RenderObject::new(mesh, mat);
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            parent: Some(ina_transform),
            ..Default::default()
        });

        world.push((transform_id, obj));
    }

    //  lost empires
    let le_transform = transform_sys.add_transform(TransformCreateInfo::default());
    for mesh in le_meshes {
        let le_obj = RenderObject::new(mesh, le_mat.clone());
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            parent: Some(le_transform),
            ..Default::default()
        });

        let mat_swapper =
            MaterialSwapper::new([le_mat.clone(), le_uv_mat.clone(), ina_mats[1].clone()]);

        world.push((transform_id, le_obj, mat_swapper));
    }

    suzanne
}

pub struct MaterialSwapper {
    materials: Vec<RenderSubmit>,
    curent_index: usize,
}
impl MaterialSwapper {
    fn new(materials: impl IntoIterator<Item = RenderSubmit>) -> Self {
        let materials = materials.into_iter().map(|m| m.into()).collect();
        Self {
            materials,
            curent_index: 0,
        }
    }

    pub fn swap_material(&mut self) -> RenderSubmit {
        self.curent_index = (self.curent_index + 1) % self.materials.len();
        self.materials[self.curent_index].clone()
    }
}
