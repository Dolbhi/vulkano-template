use std::f32::consts::PI;

use cgmath::{
    num_traits::clamp, Angle, Matrix4, One, Quaternion, Rad, Rotation, Rotation3, Vector3,
};

const CAM_SPEED: f32 = 2.;
const MOUSE_SENSITIVITY: f32 = 0.01;

pub struct Camera {
    pub fov: Rad<f32>,
    pub rotation: Quaternion<f32>,
    pub position: Vector3<f32>,
}

impl Camera {
    fn forward_vector(&self) -> Vector3<f32> {
        self.right_vector().cross((0., -1., 0.).into())
        // Matrix3::from_angle_y(self.rotation.y) * Vector3::new(0., 0., -1.)
    }
    fn right_vector(&self) -> Vector3<f32> {
        self.rotation.conjugate().rotate_vector((1., 0., 0.).into())
        // Matrix3::from_angle_y(self.rotation.y) * Vector3::new(1., 0., 0.)
    }

    pub fn move_up(&mut self, seconds_passed: f32) {
        self.position[1] += seconds_passed * CAM_SPEED;
    }
    pub fn move_down(&mut self, seconds_passed: f32) {
        self.position[1] -= seconds_passed * CAM_SPEED;
    }

    pub fn move_right(&mut self, seconds_passed: f32) {
        self.position += seconds_passed * CAM_SPEED * self.right_vector();
    }
    pub fn move_left(&mut self, seconds_passed: f32) {
        self.position -= seconds_passed * CAM_SPEED * self.right_vector();
    }

    pub fn move_forward(&mut self, seconds_passed: f32) {
        self.position += seconds_passed * CAM_SPEED * self.forward_vector();
    }
    pub fn move_back(&mut self, seconds_passed: f32) {
        self.position -= seconds_passed * CAM_SPEED * self.forward_vector();
    }

    pub fn rotate(&mut self, dx: f32, dy: f32) {
        let old_pitch = Rad::atan(self.rotation.v.x / self.rotation.s);
        let delta_pitch = clamp(
            Rad(dy * MOUSE_SENSITIVITY),
            Rad(-PI / 4.) - old_pitch,
            Rad(PI / 4.) - old_pitch,
        );

        self.rotation = Quaternion::from_angle_x(delta_pitch)
            * self.rotation
            * Quaternion::from_angle_y(Rad(dx * MOUSE_SENSITIVITY));
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::from(self.rotation) * Matrix4::from_translation(-self.position)
    }
    pub fn projection_matrix(&self, aspect: f32) -> Matrix4<f32> {
        let mut projection = cgmath::perspective(self.fov, aspect, 0.1, 200.);
        projection.y.y *= -1.;
        projection
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: Rad(1.2),
            rotation: Quaternion::one(),
            position: Vector3 {
                x: 0.,
                y: 0.,
                z: 1.,
            },
        }
    }
}
