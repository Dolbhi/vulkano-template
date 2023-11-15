use std::f32::consts::PI;

use cgmath::{num_traits::clamp, Euler, Matrix3, Matrix4, Rad, Vector3};

const CAM_SPEED: f32 = 1.3;
const MOUSE_SENSITIVITY: f32 = 0.01;

pub struct Camera {
    pub fov: Rad<f32>,
    pub rotation: Euler<Rad<f32>>,
    pub position: Vector3<f32>,
}

impl Camera {
    fn forward_vector(&self) -> Vector3<f32> {
        Matrix3::from_angle_y(-self.rotation.y) * Vector3::new(0., 0., -1.)
    }
    fn right_vector(&self) -> Vector3<f32> {
        Matrix3::from_angle_y(-self.rotation.y) * Vector3::new(1., 0., 0.)
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
        let Rad(old_x) = self.rotation.x;

        self.rotation.x = Rad(clamp(old_x + dy * MOUSE_SENSITIVITY, -PI / 2., PI / 2.));
        self.rotation.y += Rad(dx * MOUSE_SENSITIVITY);
    }

    pub fn rotation_matrix(&self) -> Matrix4<f32> {
        Matrix4::from(self.rotation)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: Rad(1.2),
            rotation: Euler::new(Rad(0.), Rad(0.), Rad(0.)),
            position: Vector3 {
                x: 0.,
                y: 0.,
                z: 0.,
            },
        }
    }
}
