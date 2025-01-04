mod collider;
mod contact;
mod geo_alg;
// mod geo_alg_com;

use std::sync::{atomic::AtomicUsize, Arc};

pub use collider::{ColliderSystem, CuboidCollider, LeafInHierachy};

use cgmath::Vector3;

use crate::game_objects::transform::{Transform, TransformID};

type Vector = Vector3<f32>;

const GRAVITY: Vector = Vector {
    x: 0.,
    y: -9.81,
    z: 0.,
};

pub struct RigidBody {
    pub transform: TransformID,
    pub velocity: Vector,
    pub bivelocity: Vector,
    pub contact_refs: Vec<Arc<AtomicUsize>>,
}
impl RigidBody {
    pub fn update(&mut self, transform: &mut Transform, delta_secs: f32) {
        self.velocity += GRAVITY * delta_secs;

        transform.mutate(|t, r, _| {
            *t += self.velocity * delta_secs;
            *r = geo_alg::bivec_exp(delta_secs * self.bivelocity).into_quaternion() * *r;
        });
    }

    pub fn point_velocity(&self, point: Vector) -> Vector {
        self.velocity + point.cross(self.bivelocity)
    }
}
