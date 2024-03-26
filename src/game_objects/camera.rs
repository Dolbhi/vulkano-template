use std::f32::consts::PI;

use cgmath::{num_traits::clamp, Angle, Matrix4, Quaternion, Rad, Rotation3};

use super::transform::{TransformID, TransformView};

// const CAM_SPEED: f32 = 2.;
const MOUSE_SENSITIVITY: f32 = 0.01;

pub struct Camera {
    pub fov: Rad<f32>,
    pub transform: TransformID,
}

impl Camera {
    pub fn rotate(rotation: &Quaternion<f32>, dx: f32, dy: f32) -> Quaternion<f32> {
        let old_pitch = Rad::atan(rotation.v.x / rotation.s);
        let delta_pitch = clamp(
            Rad(-dy * MOUSE_SENSITIVITY),
            Rad(-PI / 4.01) - old_pitch,
            Rad(PI / 4.01) - old_pitch,
        );

        Quaternion::from_angle_y(Rad(-dx * MOUSE_SENSITIVITY))
            * rotation
            * Quaternion::from_angle_x(delta_pitch)
    }

    pub fn view_matrix(&self, transform: TransformView) -> Matrix4<f32> {
        Matrix4::from(transform.rotation.conjugate().clone())
            * Matrix4::from_translation(-transform.translation.clone())
    }
    pub fn projection_matrix(&self, aspect: f32) -> Matrix4<f32> {
        let mut projection = cgmath::perspective(self.fov, aspect, 0.05, 200.);
        projection.y.y *= -1.;
        projection
    }
}
