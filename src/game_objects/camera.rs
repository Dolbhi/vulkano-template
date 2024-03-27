use std::f32::consts::PI;

use cgmath::{num_traits::clamp, Angle, Matrix4, One, Quaternion, Rad, Rotation3, Vector3, Zero};

use super::{
    transform::{TransformID, TransformSystem},
    utility::VectorDamp,
};

// const CAM_SPEED: f32 = 2.;
const MOUSE_SENSITIVITY: f32 = 0.01;

pub struct Camera {
    pub fov: Rad<f32>,
    rotation: Quaternion<f32>,
    smooth_pos: Vector3<f32>,
    pub transform: TransformID,
    damper: VectorDamp,
}

impl Camera {
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        let old_pitch = Rad::atan(self.rotation.v.x / self.rotation.s);
        let delta_pitch = clamp(
            Rad(-dy * MOUSE_SENSITIVITY),
            Rad(-PI / 4.01) - old_pitch,
            Rad(PI / 4.01) - old_pitch,
        );

        self.rotation = Quaternion::from_angle_y(Rad(-dx * MOUSE_SENSITIVITY))
            * self.rotation
            * Quaternion::from_angle_x(delta_pitch);
    }
    /// Lerp camera slightly towards target position
    ///
    /// Update transform rotation to match camera
    pub fn sync_transform(&mut self, system: &mut TransformSystem) {
        let transform = system.get_transform_mut(&self.transform).unwrap();
        let view = transform.get_local_transform();
        self.smooth_pos = self
            .damper
            .smooth_follow(self.smooth_pos, *view.translation); //self.smooth_pos.lerp(*view.translation, 0.125);
        transform.set_rotation(self.rotation);
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        // Matrix4::from(transform.rotation.conjugate().clone())
        //     * Matrix4::from_translation(-transform.translation.clone())
        Matrix4::from(self.rotation.conjugate().clone())
            * Matrix4::from_translation(-self.smooth_pos)
    }
    pub fn projection_matrix(&self, aspect: f32) -> Matrix4<f32> {
        let mut projection = cgmath::perspective(self.fov, aspect, 0.05, 200.);
        projection.y.y *= -1.;
        projection
    }

    pub fn from_transform(transform: TransformID) -> Self {
        Self {
            fov: Rad(1.2),
            rotation: Quaternion::one(),
            smooth_pos: Vector3::zero(),
            transform,
            damper: VectorDamp::new(40.),
        }
    }
}
