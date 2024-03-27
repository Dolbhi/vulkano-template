use std::time::Instant;

use cgmath::{Vector3, Zero};

pub struct VectorDamp {
    last_time: Instant,
    current_velocity: Vector3<f32>,
    strength: f32,
}
impl VectorDamp {
    pub fn new(strength: f32) -> Self {
        Self {
            last_time: Instant::now(),
            current_velocity: Vector3::zero(),
            strength,
        }
    }

    pub fn smooth_follow(&mut self, current: Vector3<f32>, target: Vector3<f32>) -> Vector3<f32> {
        let difference = current - target;

        self.current_velocity -=
            self.strength * (2. * self.current_velocity + self.strength * difference);

        let elapsed_time = self.last_time.elapsed().as_secs_f32();
        self.last_time = Instant::now();

        current + self.current_velocity * elapsed_time
    }
}
