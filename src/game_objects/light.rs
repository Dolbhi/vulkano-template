use cgmath::{Vector3, Vector4};

use crate::shaders::lighting::fs::DirectionLight;

pub struct PointLightComponent {
    pub color: Vector4<f32>,
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
