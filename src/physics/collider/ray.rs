use super::BoundingBox;
use crate::physics::{matrix_truncate, Vector};
use cgmath::{InnerSpace, Matrix4, Zero};

#[derive(Clone)]
pub struct Ray {
    pub origin: Vector,
    pub direction: Vector,
    pub distance: f32,
}

impl Ray {
    /// creates ray with normalised direction
    pub fn new(origin: Vector, direction: Vector, distance: f32) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
            distance,
        }
    }

    /// creates ray with normalised direction
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

    /// Assumes model is normalised
    ///
    /// Resulting ray is not normalised
    pub fn transform_model(&mut self, model: &Matrix4<f32>) {
        self.origin += model.w.truncate();
        // self.origin = (model * self.origin.extend(1.)).truncate();

        let rotation = matrix_truncate(model);
        self.origin = rotation * self.origin;
        self.direction = rotation * self.direction;

        // let end = self.calc_point(self.distance);
        // let end = (model * end.extend(1.)).truncate();
        // self.direction = (end - self.origin) / self.distance;

        // let scale = 1. / self.direction.magnitude();
        // self.direction *= scale;
        // self.distance *= scale;
    }

    /// convert distance to point on ray
    pub fn calc_point(&self, distance: f32) -> Vector {
        self.origin + self.direction * distance
    }

    pub fn advance(&mut self, distance: f32) {
        self.origin += self.direction * distance;
        self.distance -= distance;
    }

    /// Gives distance to intercept point on cuboid
    pub fn cuboid_intersection(&self, inv_cuboid_model: &Matrix4<f32>) -> Option<f32> {
        let mut cub_space_ray = self.clone();
        cub_space_ray.transform_model(inv_cuboid_model);

        cub_space_ray.box_intersection(&BoundingBox::new((-1., -1., -1.), (1., 1., 1.)))
        // let (close, far) =
        //     cub_space_ray.box_intersection_raw(&BoundingBox::new((-1., -1., -1.), (1., 1., 1.)));

        // if close <= far && close <= self.distance {
        //     if close < 0. {
        //         if far < 0. {
        //             // intercepting completely behind ray
        //             None
        //         } else {
        //             // ray origin in box
        //             Some(0.)
        //         }
        //     } else {
        //         // normal interception
        //         Some(close)
        //     }
        // } else {
        //     None
        // }
    }

    /// only gives result on valid intersect and only returns distance to entering intersect
    pub fn box_intersection(&self, bounds: &BoundingBox) -> Option<f32> {
        let (close, far) = self.box_intersection_raw(bounds);

        if close <= far && close <= self.distance {
            if close < 0. {
                if far < 0. {
                    // intercepting completely behind ray
                    None
                } else {
                    // ray origin in box
                    Some(0.)
                }
            } else {
                // normal interception
                Some(close)
            }
        } else {
            None
        }
    }

    pub fn box_intersection_raw(&self, bounds: &BoundingBox) -> (f32, f32) {
        let mut closest = f32::NEG_INFINITY;
        let mut furthest = f32::INFINITY;

        for i in 0..3 {
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
