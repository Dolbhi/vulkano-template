use std::{f32::consts::PI, iter::zip};

use cgmath::{InnerSpace, Quaternion, Rad, Rotation3, Vector3};

use crate::{
    game_objects::{
        light::PointLightComponent, transform::TransformCreateInfo, MaterialSwapper, Rotate,
        WorldLoader,
    },
    physics::RigidBody,
    render::resource_manager::{MaterialID::*, MeshID::*, TextureID},
};

// type Mesh = std::sync::Arc<crate::vulkano_objects::buffers::MeshBuffers<VertexFull>>;

pub fn init_world(mut loader: WorldLoader) {
    // meshes
    let ina_meshes = [InaBody, InaCloth, InaHair, InaHead];
    let le_meshes = (0..45u8).map(|n| LostEmpire(n));

    // materials
    let ina_mats = [
        TextureID::InaBody,
        TextureID::InaCloth,
        TextureID::InaHair,
        TextureID::InaHead,
    ]
    .map(|id| Texture(id));
    // colored materials
    let red_mat = loader.2.load_solid_material([1., 0., 0., 1.], false).0;
    let blue_mat = loader.2.load_solid_material([0., 0., 1., 1.], false).0;
    let green_mat = loader.2.load_solid_material([0., 1., 0., 1.], true).0;

    // objects
    //      Suzanne
    let suzanne_obj = loader.2.load_ro(Suzanne, UV, true);
    let rotate = Rotate(Vector3::new(1.0, 1.0, 0.0).normalize(), Rad(5.0));
    loader.add_2_comp([0., 0., 0.], suzanne_obj, rotate);

    //      Spam Suzanne
    for x in 0..20 {
        for z in 0..20 {
            let mat = [ina_mats[1], green_mat][(x + z) % 2];
            loader.quick_ro([x as f32, 7f32, z as f32], Suzanne, mat, true);
        }
    }

    //      Squares
    let obj = loader.2.load_ro(Square, Gradient, false);
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        loader.add_1_comp([x, y, z], obj.clone());
    }

    //      Ina
    let rotate = Rotate([0., 1., 0.].into(), Rad(0.5));
    let (ina_transform, _) = loader.add_1_comp([0.0, 5.0, -1.0], rotate);
    for (mesh, mat) in zip(ina_meshes, ina_mats.clone()) {
        loader.quick_ro(ina_transform, mesh, mat, true);
    }

    //      lost empires
    let le_transform = loader.1.add_transform(TransformCreateInfo::default());
    for mesh in le_meshes {
        let le_obj = loader.2.load_ro(mesh, Texture(TextureID::LostEmpire), true);
        let mat_swapper = MaterialSwapper::new(
            [
                (Texture(TextureID::LostEmpire), true),
                (Texture(TextureID::LostEmpire), false),
                (UV, false),
                (ina_mats[1], true),
            ]
            .map(|(id, lit)| loader.2.get_material(id, lit)),
        );

        loader.add_2_comp(le_transform, le_obj, mat_swapper);
    }

    // lights
    let ro = loader.2.load_ro(Cube, red_mat, false);
    loader.add_2_comp(
        TransformCreateInfo::from([0., 5., -1.]).set_scale([0.1, 0.1, 0.1]),
        PointLightComponent::new([1., 0., 0., 3.], 3.),
        ro,
    );
    let ro = loader.2.load_ro(Cube, blue_mat, false);
    loader.add_2_comp(
        TransformCreateInfo::from([0.0, 6.0, -0.5]).set_scale([0.1, 0.1, 0.1]),
        PointLightComponent::new([0., 0., 1., 2.], 3.),
        ro,
    );

    // spam lights
    let ro = loader.2.load_ro(Cube, red_mat, false);
    for x in 0..20 {
        for z in -10..10 {
            loader.add_2_comp(
                TransformCreateInfo::from([x as f32, 6.1, z as f32]).set_scale([0.1, 0.1, 0.1]),
                PointLightComponent::new([1., 0., 0., 1.], 1.),
                ro.clone(),
            );
        }
    }
}

/// Empty scene with just the lost empire map
pub fn init_ui_test(mut loader: WorldLoader) {
    for i in 0..45u8 {
        loader.quick_ro(
            [0., 0., 0.],
            LostEmpire(i),
            Texture(TextureID::LostEmpire),
            true,
        );
    }
}

/// Large plane + cube
pub fn init_phys_test(mut loader: WorldLoader) {
    // let plane_mesh = resources.get_mesh(Square);
    // let cube_mesh = resources.get_mesh(Cube);

    let yellow_mat = loader.2.load_solid_material([1., 1., 0., 1.], true).0;
    let green_mat = loader.2.load_solid_material([0., 1., 0., 1.], true).0;

    let plane_trans = TransformCreateInfo {
        rotation: Quaternion::from_axis_angle([1., 0., 0.].into(), Rad(-PI / 2.)),
        scale: [10., 10., 1.].into(),
        ..Default::default()
    };
    loader.quick_ro(plane_trans, Square, yellow_mat, true);

    let ro = loader.2.load_ro(Cube, green_mat, true);
    let rb = RigidBody {
        velocity: (1.0, 10.0, 0.0).into(),
        bivelocity: (0.0, 0.0, -5.0).into(),
    };
    loader.add_2_comp([0., 1., 0.], ro, rb);
}
