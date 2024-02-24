pub mod app;
pub mod game_objects;
pub mod render;
pub mod shaders;
mod vertex_data;
pub mod vulkano_objects;

use cgmath::{Vector3, Vector4};
pub use vertex_data::{Vertex2d, Vertex3d, VertexFull};
use vulkano::{buffer::BufferUsage, descriptor_set::WriteDescriptorSet};

use crate::{
    game_objects::{light::PointLightComponent, transform::TransformCreateInfo},
    render::{mesh::from_obj, RenderObject},
    shaders::draw,
};

use game_objects::transform::{TransformID, TransformSystem};
use legion::World;
use render::{Context, DeferredRenderer, DrawSystem, RenderSubmit};
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
    lit_system: &mut DrawSystem<{ DeferredRenderer::LIT }>,
    unlit_system: &mut DrawSystem<{ DeferredRenderer::UNLIT }>,
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
    let basic_shader = &mut lit_system.shaders[0];
    let [solid_shader, uv_shader, grad_shader] = &mut unlit_system.shaders;

    let le_mat = resource_loader.init_material(
        basic_shader,
        [WriteDescriptorSet::image_view_sampler(
            0,
            le_texture.clone(),
            linear_sampler.clone(),
        )],
    );
    let le_uv_mat = uv_shader.add_material(None);

    //  ina
    let ina_mats: Vec<_> = zip(["hair", "cloth", "body", "head"], ina_textures)
        .map(|(_, tex)| {
            resource_loader.init_material(
                basic_shader,
                [WriteDescriptorSet::image_view_sampler(
                    0,
                    tex,
                    linear_sampler.clone(),
                )],
            )
        })
        .collect();

    //  uv
    let uv_mat = uv_shader.add_material(None);

    //  grad
    let grad_mat = grad_shader.add_material(None);

    //  unlit solids
    let red_material = {
        let red_mat_buffer = resource_loader.create_material_buffer(
            draw::SolidData {
                color: [1., 0., 0., 1.],
            },
            BufferUsage::empty(),
        );
        resource_loader.init_material(
            solid_shader,
            [WriteDescriptorSet::buffer(0, red_mat_buffer)],
        )
        // solid_pipeline.add_material(Some(resource_loader.load_material_set(
        //     &solid_pipeline,
        //     [WriteDescriptorSet::buffer(0, red_mat_buffer)],
        // )))
    };
    let blue_material = {
        let blue_mat_buffer = resource_loader.create_material_buffer(
            draw::SolidData {
                color: [0., 0., 1., 1.],
            },
            BufferUsage::empty(),
        );
        resource_loader.init_material(
            solid_shader,
            [WriteDescriptorSet::buffer(0, blue_mat_buffer)],
        )
        // solid_pipeline.add_material(Some(resource_loader.load_material_set(
        //     &solid_pipeline,
        //     [WriteDescriptorSet::buffer(0, blue_mat_buffer)],
        // )))
    };

    // meshes
    //      suzanne
    let suzanne_mesh = {
        let (vertices, indices) = from_obj(Path::new("models/suzanne.obj")).pop().unwrap();
        resource_loader.load_mesh(vertices, indices)
    };

    //      square
    let square = {
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
        resource_loader.load_mesh(vertices, indices)
    };

    //      cube
    let cube_mesh = {
        let (vertices, indices) = from_obj(Path::new("models/default_cube.obj"))
            .pop()
            .expect("Failed to load cube mesh");
        resource_loader.load_mesh(vertices, indices)
    };

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
    //      Suzanne
    let suzanne_obj = RenderObject::new(suzanne_mesh.clone(), uv_mat.clone());
    let suzanne = transform_sys.next().unwrap();
    world.push((suzanne, suzanne_obj));

    //      Spam Suzanne
    for x in 0..20 {
        for z in 0..20 {
            let square_obj = RenderObject::new(suzanne_mesh.clone(), ina_mats[1].clone());
            let transform_id = transform_sys.add_transform(TransformCreateInfo {
                translation: [x as f32, 7f32, z as f32].into(),
                ..Default::default()
            });

            world.push((transform_id, square_obj));
        }
    }

    //      Squares
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        let square_obj = RenderObject::new(square.clone(), grad_mat.clone()); //uv_mat.clone());
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            translation: [x, y, z].into(),
            ..Default::default()
        });

        world.push((transform_id, square_obj));
    }

    //      Ina
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

    //      lost empires
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

    // lights
    world.push((
        transform_sys.add_transform(TransformCreateInfo {
            scale: Vector3::new(0.1, 0.1, 0.1),
            translation: Vector3::new(0., 5., -1.),
            ..Default::default()
        }),
        PointLightComponent {
            color: Vector4::new(1., 0., 0., 3.),
            half_radius: 3.,
        },
        RenderObject::new(cube_mesh.clone(), red_material.clone()),
    ));
    world.push((
        transform_sys.add_transform(TransformCreateInfo {
            scale: Vector3::new(0.1, 0.1, 0.1),
            translation: Vector3::new(0.0, 6.0, -0.5),
            ..Default::default()
        }),
        PointLightComponent {
            color: Vector4::new(0., 0., 1., 2.),
            half_radius: 3.,
        },
        RenderObject::new(cube_mesh.clone(), blue_material),
    ));

    // spam lights
    for x in 0..20 {
        for z in -10..10 {
            world.push((
                transform_sys.add_transform(TransformCreateInfo {
                    scale: Vector3::new(0.1, 0.1, 0.1),
                    translation: Vector3::new(x as f32, 6.1, z as f32),
                    ..Default::default()
                }),
                PointLightComponent {
                    color: Vector4::new(1., 0., 0., 1.),
                    half_radius: 1.,
                },
                RenderObject::new(cube_mesh.clone(), red_material.clone()),
            ));
        }
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
