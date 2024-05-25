use std::{f32::consts::PI, iter::zip};

use cgmath::{InnerSpace, Quaternion, Rad, Rotation3, Vector3, Vector4};
use legion::World;

use crate::{
    game_objects::{
        light::PointLightComponent,
        transform::{TransformCreateInfo, TransformSystem},
        MaterialSwapper, Rotate,
    },
    physics::RigidBody,
    render::{
        resource_manager::{MaterialID, MeshID, ResourceRetriever, TextureID},
        RenderObject,
    },
    vertex_data::VertexFull,
};

type Mesh = std::sync::Arc<crate::vulkano_objects::buffers::MeshBuffers<VertexFull>>;

pub fn init_world(
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

    let red_mat = resources.load_solid_material([1., 0., 0., 1.], false).2;
    let blue_mat = resources.load_solid_material([0., 0., 1., 1.], false).2;

    let green_mat = resources.load_solid_material([0., 1., 0., 1.], true).2;

    // objects
    //      Suzanne
    let suzanne = transform_sys.next().unwrap();
    let suzanne_obj = RenderObject::new_default_data(suzanne_mesh.clone(), uv_mat.clone());
    let rotate = Rotate(Vector3::new(1.0, 1.0, 0.0).normalize(), Rad(5.0));
    world.push((suzanne, suzanne_obj, rotate));

    //      Spam Suzanne
    for x in 0..20 {
        for z in 0..20 {
            let mat = if (x + z) % 2 == 0 {
                &ina_mats[1]
            } else {
                &green_mat
            };
            let square_obj = RenderObject::new_default_data(suzanne_mesh.clone(), mat.clone());
            let transform_id = transform_sys.add_transform(TransformCreateInfo {
                translation: [x as f32, 7f32, z as f32].into(),
                ..Default::default()
            });

            world.push((transform_id, square_obj));
        }
    }

    //      Squares
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        let square_obj = RenderObject::new_default_data(square_mesh.clone(), grad_mat.clone()); //uv_mat.clone());
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
        let obj = RenderObject::new_default_data(mesh, mat);
        let transform_id = transform_sys.add_transform(TransformCreateInfo {
            parent: Some(ina_transform),
            ..Default::default()
        });

        world.push((transform_id, obj));
    }

    //      lost empires
    let le_transform = transform_sys.add_transform(TransformCreateInfo::default());
    for mesh in le_meshes {
        let le_obj = RenderObject::new_default_data(mesh, le_mat.clone());
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
        RenderObject::new_default_data(cube_mesh.clone(), red_mat.clone()),
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
        RenderObject::new_default_data(cube_mesh.clone(), blue_mat),
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
                RenderObject::new_default_data(cube_mesh.clone(), red_mat.clone()),
            ));
        }
    }
}

/// Empty scene with just the lost empire map
pub fn init_ui_test(
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
        let le_obj = RenderObject::new_default_data(mesh, le_mat.clone());
        world.push((transform_id, le_obj));
    }
}

/// Large plane + cube
pub fn init_phys_test(
    world: &mut World,
    transform_sys: &mut TransformSystem,
    resources: &mut ResourceRetriever,
) {
    let plane_mesh = resources.get_mesh(MeshID::Square);
    let cube_mesh = resources.get_mesh(MeshID::Cube);

    let yellow_mat = resources.load_solid_material([1., 1., 0., 1.], true).2;
    let green_mat = resources.load_solid_material([0., 1., 0., 1.], true).2;

    let plane_trans = transform_sys.add_transform(TransformCreateInfo {
        rotation: Quaternion::from_axis_angle([1., 0., 0.].into(), Rad(-PI / 2.)),
        scale: [10., 10., 1.].into(),
        ..Default::default()
    });
    world.push((
        plane_trans,
        RenderObject::new_default_data(plane_mesh, yellow_mat),
    ));

    let cube_trans =
        transform_sys.add_transform(TransformCreateInfo::default().set_translation([0., 1., 0.]));
    let rigid_body = RigidBody {
        velocity: (1.0, 10.0, 0.0).into(),
        bivelocity: (0.0, 0.0, -5.0).into(),
    };
    world.push((
        cube_trans,
        RenderObject::new_default_data(cube_mesh, green_mat),
        rigid_body,
    ));
}
