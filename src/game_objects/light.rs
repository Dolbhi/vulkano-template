use cgmath::{Vector3, Vector4};

use crate::shaders::lighting::{DirectionLight, PointLight};

#[derive(Clone)]
pub struct PointLightComponent {
    pub color: Vector4<f32>,
    pub half_radius: f32,
}

impl PointLightComponent {
    pub fn into_light(self, position: Vector3<f32>) -> PointLight {
        PointLight {
            color: self.color.into(),
            position: position.extend(self.half_radius * 2.).into(),
        }
    }
}

pub struct DirectionalLightComponent {
    pub color: Vector4<f32>,
    pub direction: Vector3<f32>,
}

impl Into<DirectionLight> for DirectionalLightComponent {
    fn into(self) -> DirectionLight {
        DirectionLight {
            color: self.color.into(),
            direction: self.direction.extend(1.).into(),
        }
    }
}
