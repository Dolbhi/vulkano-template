pub mod app;
pub mod game_objects;
pub mod profiler;
pub mod render;
pub mod shaders;
pub mod ui;
mod vertex_data;
pub mod vulkano_objects;

use cgmath::{Quaternion, Rad, Rotation3, Vector3, Vector4};
use profiler::Profiler;
pub use vertex_data::{Vertex2d, Vertex3d, VertexFull};

use crate::{
    game_objects::{light::PointLightComponent, transform::TransformCreateInfo},
    render::RenderObject,
};

use game_objects::{transform::TransformSystem, Rotate};
use legion::World;
use render::{
    resource_manager::{MaterialID, MeshID, ResourceRetriever, TextureID},
    RenderSubmit,
};
use std::{f32::consts::PI, iter::zip};

pub static mut RENDER_PROFILER: Option<Profiler<7, 128>> = Some(Profiler::new([
    "GUI and inputs",
    "Pre-render",
    "Frame cleanup",
    "Render upload",
    "Wait last frame",
    "ComBuf building",
    "Execute",
]));
pub static mut LOGIC_PROFILER: Option<Profiler<2, 128>> =
    Some(Profiler::new(["Lock wait", "Logic update"]));

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

type Mesh = std::sync::Arc<vulkano_objects::buffers::Buffers<VertexFull>>;

fn init_world(
    world: &mut World,
    transform_sys: &mut TransformSystem,
    resources: &mut ResourceRetriever,
) {
    // let le_mesh_count = from_obj(Path::new("models/lost_empire.obj")).len(); // 45

    // meshes
    let suzanne_mesh = resources.get_mesh(MeshID::Suzanne);
    let square_mesh = resources.get_mesh(MeshID::Square);
    let cube_mesh = resources.get_mesh(MeshID::Cube);

    let ina_meshes = [
        MeshID::InaBody,
        MeshID::InaCloth,
        MeshID::InaHair,
        MeshID::InaHead,
    ]
    .map(|id| resources.get_mesh(id));

    let le_meshes: Vec<Mesh> = (0..45u8)
        .map(|n| resources.get_mesh(MeshID::LostEmpire(n)))
        .collect();

    // materials
    let uv_mat = resources.get_material(MaterialID::UV, false);
    let grad_mat = resources.get_material(MaterialID::Gradient, false);

    let ina_mats = [
        TextureID::InaBody,
        TextureID::InaCloth,
        TextureID::InaHair,
        TextureID::InaHead,
    ]
    .map(|id| resources.get_material(MaterialID::Texture(id), true));

    let le_mat = resources.get_material(MaterialID::Texture(TextureID::LostEmpire), true);
    let le_mat_unlit = resources.get_material(MaterialID::Texture(TextureID::LostEmpire), false);

    let red_mat = resources.get_material(MaterialID::Color([u8::MAX, 0, 0, u8::MAX]), false);
    let blue_mat = resources.get_material(MaterialID::Color([0, 0, u8::MAX, u8::MAX]), false);

    let green_mat = resources.get_material(MaterialID::Color([0, u8::MAX, 0, u8::MAX]), true);

    // objects
    //      Suzanne
    let suzanne = transform_sys.next().unwrap();
    let suzanne_obj = RenderObject::new(suzanne_mesh.clone(), uv_mat.clone());
    let rotate = Rotate([0., 1., 0.].into(), Rad(1.0));
    world.push((suzanne, suzanne_obj, rotate));

    //      Spam Suzanne
    for x in 0..20 {
        for z in 0..20 {
            let mat = if (x + z) % 2 == 0 {
                &ina_mats[1]
            } else {
                &green_mat
            };
            let square_obj = RenderObject::new(suzanne_mesh.clone(), mat.clone());
            let transform_id = transform_sys.add_transform(TransformCreateInfo {
                translation: [x as f32, 7f32, z as f32].into(),
                ..Default::default()
            });

            world.push((transform_id, square_obj));
        }
    }

    //      Squares
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        let square_obj = RenderObject::new(square_mesh.clone(), grad_mat.clone()); //uv_mat.clone());
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
    let rotate = Rotate([0., 1., 0.].into(), Rad(0.5));
    world.push((ina_transform, rotate));
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

        let mat_swapper = MaterialSwapper::new([
            le_mat.clone(),
            le_mat_unlit.clone(),
            uv_mat.clone(),
            ina_mats[1].clone(),
        ]);

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
        RenderObject::new(cube_mesh.clone(), red_mat.clone()),
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
        RenderObject::new(cube_mesh.clone(), blue_mat),
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
                RenderObject::new(cube_mesh.clone(), red_mat.clone()),
            ));
        }
    }
}

/// Empty scene with just the lost empire map
fn init_ui_test(
    world: &mut World,
    transform_sys: &mut TransformSystem,
    resources: &mut ResourceRetriever,
) {
    let le_meshes: Vec<Mesh> = (0..45u8)
        .map(|n| resources.get_mesh(MeshID::LostEmpire(n)))
        .collect();

    let le_mat = resources.get_material(MaterialID::Texture(TextureID::LostEmpire), true);

    let le_root = transform_sys.add_transform(TransformCreateInfo::default());
    for mesh in le_meshes {
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            parent: Some(le_root),
            ..Default::default()
        });
        let le_obj = RenderObject::new(mesh, le_mat.clone());
        world.push((transform_id, le_obj));
    }
}

/// Large plane + cube
fn init_phys_test(
    world: &mut World,
    transform_sys: &mut TransformSystem,
    resources: &mut ResourceRetriever,
) {
    let plane_mesh = resources.get_mesh(MeshID::Square);
    let cube_mesh = resources.get_mesh(MeshID::Cube);

    let yellow_mat = resources.get_material(MaterialID::Color([255, 255, 0, 255]), true);
    let green_mat = resources.get_material(MaterialID::Color([0, 255, 0, 255]), true);

    let plane_trans = transform_sys.add_transform(TransformCreateInfo {
        rotation: Quaternion::from_axis_angle([1., 0., 0.].into(), Rad(-PI / 2.)),
        scale: [10., 10., 1.].into(),
        ..Default::default()
    });
    world.push((plane_trans, RenderObject::new(plane_mesh, yellow_mat)));

    let cube_trans =
        transform_sys.add_transform(TransformCreateInfo::default().set_translation([0., 1., 0.]));
    world.push((cube_trans, RenderObject::new(cube_mesh, green_mat)));
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
