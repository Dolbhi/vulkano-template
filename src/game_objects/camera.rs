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
        Matrix3::from_angle_y(self.rotation.y) * Vector3::new(0., 0., -1.)
    }
    fn right_vector(&self) -> Vector3<f32> {
        Matrix3::from_angle_y(self.rotation.y) * Vector3::new(1., 0., 0.)
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
        self.rotation.x = Rad(clamp(
            self.rotation.x.0 - dy * MOUSE_SENSITIVITY,
            -PI / 2.,
            PI / 2.,
        ));
        self.rotation.y += Rad(-dx * MOUSE_SENSITIVITY);
    }

    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::from(Euler::new(-self.rotation.x, -self.rotation.y, Rad(0.)))
            * Matrix4::from_translation(-self.position)
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
            rotation: Euler::new(Rad(0.), Rad(0.), Rad(0.)),
            position: Vector3 {
                x: 0.,
                y: 0.,
                z: 1.,
            },
        }
    }
}
