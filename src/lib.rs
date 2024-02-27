pub mod app;
pub mod game_objects;
pub mod render;
pub mod shaders;
mod vertex_data;
pub mod vulkano_objects;

use cgmath::{Vector3, Vector4};
pub use vertex_data::{Vertex2d, Vertex3d, VertexFull};

use crate::game_objects::{light::PointLightComponent, transform::TransformCreateInfo};

use game_objects::{
    object_loader::{ComponentInfo, ObjectInfo, ObjectLoader},
    transform::TransformID,
};
use render::{
    resource_manager::{MaterialID, MeshID, TextureID},
    RenderSubmit,
};
use std::iter::zip;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

fn init_render_objects(mut object_loader: ObjectLoader) -> TransformID {
    let ina_meshes = [
        MeshID::InaBody,
        MeshID::InaCloth,
        MeshID::InaHair,
        MeshID::InaHead,
    ];

    let ina_mats = [
        TextureID::InaBody,
        TextureID::InaCloth,
        TextureID::InaHair,
        TextureID::InaHead,
    ];

    // objects
    //      Suzanne
    let suzanne = object_loader.create_object(ObjectInfo {
        components: vec![ComponentInfo::Render(MeshID::Suzanne, MaterialID::UV)],
        ..Default::default()
    });

    //      Spam Suzanne
    let ro_info =
        ComponentInfo::Render(MeshID::Suzanne, MaterialID::LitTexture(TextureID::InaCloth));
    for x in 0..20 {
        for z in 0..20 {
            object_loader.create_object(ObjectInfo {
                transform: TransformCreateInfo {
                    translation: [x as f32, 7f32, z as f32].into(),
                    ..Default::default()
                },
                components: vec![ro_info.clone()],
                ..Default::default()
            });
        }
    }

    //      Squares
    let ro_info = ComponentInfo::Render(MeshID::Square, MaterialID::Gradient);
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        object_loader.create_object(ObjectInfo {
            transform: TransformCreateInfo {
                translation: [x, y, z].into(),
                ..Default::default()
            },
            components: vec![ro_info.clone()],
            ..Default::default()
        });
    }

    //      Ina
    let children = zip(ina_meshes, ina_mats)
        .map(|(mesh, tex)| ObjectInfo {
            components: vec![ComponentInfo::Render(mesh, MaterialID::LitTexture(tex))],
            ..Default::default()
        })
        .collect();
    let transform = TransformCreateInfo {
        translation: [0.0, 5.0, -1.0].into(),
        ..Default::default()
    };
    let ina = ObjectInfo {
        transform,
        children,
        ..Default::default()
    };
    object_loader.create_object(ina);

    //      lost empires
    let le_mat = MaterialID::LitTexture(TextureID::LostEmpire);
    let children = (0..45)
        .map(|n| ObjectInfo {
            components: vec![
                ComponentInfo::Render(MeshID::LostEmpire(n), le_mat),
                ComponentInfo::MaterialSwapper(vec![
                    le_mat,
                    MaterialID::LitTexture(TextureID::InaHead),
                    MaterialID::UV,
                ]),
            ],
            ..Default::default()
        })
        .collect();
    let lost_empire = ObjectInfo {
        children,
        ..Default::default()
    };
    object_loader.create_object(lost_empire);

    // lights
    // red light
    let info = ObjectInfo {
        transform: TransformCreateInfo {
            scale: Vector3::new(0.1, 0.1, 0.1),
            translation: Vector3::new(0., 5., -1.),
            ..Default::default()
        },
        components: vec![
            ComponentInfo::PointLight(PointLightComponent {
                color: Vector4::new(1., 0., 0., 3.),
                half_radius: 3.,
            }),
            ComponentInfo::Render(
                MeshID::Cube,
                MaterialID::UnlitColor([u8::MAX, 0, 0, u8::MAX]),
            ),
        ],
        ..Default::default()
    };
    object_loader.create_object(info);
    // blue light
    let info = ObjectInfo {
        transform: TransformCreateInfo {
            scale: Vector3::new(0.1, 0.1, 0.1),
            translation: Vector3::new(0.0, 6.0, -0.5),
            ..Default::default()
        },
        components: vec![
            ComponentInfo::PointLight(PointLightComponent {
                color: Vector4::new(0., 0., 1., 2.),
                half_radius: 3.,
            }),
            ComponentInfo::Render(
                MeshID::Cube,
                MaterialID::UnlitColor([0, 0, u8::MAX, u8::MAX]),
            ),
        ],
        ..Default::default()
    };
    object_loader.create_object(info);

    // spam lights
    let light_component = vec![
        ComponentInfo::PointLight(PointLightComponent {
            color: Vector4::new(1., 0., 0., 1.),
            half_radius: 1.,
        }),
        ComponentInfo::Render(
            MeshID::Cube,
            MaterialID::UnlitColor([u8::MAX, 0, 0, u8::MAX]),
        ),
    ];
    for x in 0..20 {
        for z in -10..10 {
            let transform = TransformCreateInfo {
                scale: Vector3::new(0.1, 0.1, 0.1),
                translation: Vector3::new(x as f32, 6.1, z as f32),
                ..Default::default()
            };
            object_loader.create_object(ObjectInfo {
                transform,
                components: light_component.clone(),
                ..Default::default()
            });
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
