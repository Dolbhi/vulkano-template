use super::BoundingBox;
use crate::physics::Vector;
use cgmath::{InnerSpace, Matrix4, Quaternion, Zero};

#[derive(Clone)]
pub struct Ray {
    pub origin: Vector,
    pub direction: Vector,
    pub distance: f32,
}

impl Ray {
    pub fn from_points(start: Vector, end: Vector) -> Self {
        let direction = end - start;
        let distance = direction.magnitude();
        Self {
            origin: start,
            direction: direction / distance,
            distance,
        }
    }

    // pub fn transform(&mut self, translation: Vector, rotation: Quaternion<f32>) {
    //     self.origin += translation;
    //     self.direction = rotation * self.direction;
    // }

    // /// Assumes model is normalised
    // pub fn transform_model(&mut self, model: Matrix4<f32>) {
    //     self.
    // }

    /// convert distance to point on ray
    pub fn calc_point(&self, distance: f32) -> Vector {
        self.origin + self.direction * distance
    }

    pub fn advance(&mut self, distance: f32) {
        self.origin += self.direction * distance;
        self.distance -= distance;
    }

    pub fn box_intersection(&self, bounds: &BoundingBox) -> (f32, f32) {
        let mut closest = f32::NEG_INFINITY;
        let mut furthest = f32::INFINITY;

        for i in 0..2 {
            if self.direction[i].is_zero() {
                continue;
            }

            let low = (bounds.min[i] - self.origin[i]) / self.direction[i];
            let high = (bounds.max[i] - self.origin[i]) / self.direction[i];

            let (close, far) = if low < high { (low, high) } else { (high, low) };

            if close > closest {
                closest = close
            };
            if far < furthest {
                furthest = far
            };
        }
        (closest, furthest)
    }
}
