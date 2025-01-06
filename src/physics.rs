mod collider;
mod contact;
mod geo_alg;
// mod geo_alg_com;

use crate::game_objects::transform::{Transform, TransformID};
use cgmath::{InnerSpace, Quaternion, Vector3, Zero};
pub use collider::{ColliderSystem, CuboidCollider, LeafInHierachy};
use std::sync::{atomic::AtomicUsize, Arc};

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

    pub inv_mass: f32,
    /// sqrt of masses at unit distance on principle axes
    pub sqrt_angular_mass: Vector,
    pub gravity_multiplier: f32,

    pub contact_refs: Vec<Arc<AtomicUsize>>,
}
impl RigidBody {
    pub fn new(transform: TransformID) -> Self {
        RigidBody {
            transform,
            velocity: Vector::zero(),
            bivelocity: Vector::zero(),

            inv_mass: 1.,
            sqrt_angular_mass: (1., 1., 1.).into(),
            gravity_multiplier: 1.,

            contact_refs: Vec::new(),
        }
    }

    pub fn update(&mut self, transform: &mut Transform, delta_secs: f32) {
        self.velocity += GRAVITY * delta_secs * self.gravity_multiplier;

        transform.mutate(|t, r, _| {
            *t += self.velocity * delta_secs;
            *r = geo_alg::bivec_exp((delta_secs / 2.) * self.bivelocity).into_quaternion() * *r;
        });

        // println!("MASSES: {:?}", self.sqrt_angular_mass);
    }

    pub fn point_velocity(&self, point: Vector) -> Vector {
        self.velocity + point.cross(self.bivelocity)
    }

    /// Set principle axis masses assuming object is a cuboid of constant density, taking object scale into account
    ///
    /// Does nothing if inv_mass is zero (i.e infinite mass)
    pub fn set_moi_as_cuboid(&mut self, scale: Vector) {
        if self.inv_mass.is_zero() {
            return;
        }
        self.sqrt_angular_mass = (1. / (self.inv_mass * 24.)).sqrt() * scale;
    }

    /// inverse moment of inertia about an axis (and other stuff), calculated via black magic
    pub fn angular_vel_per_impulse(
        &self,
        torque_per_impulse: Vector,
        rotation: Quaternion<f32>,
    ) -> f32 {
        let world_sam = rotation * self.sqrt_angular_mass;
        let tpi_squared = torque_per_impulse.magnitude2();

        if tpi_squared.is_zero() {
            return 0.;
        }

        (tpi_squared * tpi_squared) / torque_per_impulse.cross(world_sam).magnitude2()
    }
}

#[cfg(test)]
mod physics_tests {
    use crate::game_objects::transform::TransformSystem;
    use crate::physics::RigidBody;

    #[test]
    fn check_angular_vpi() {
        let mut transform = TransformSystem::new();
        let mut rb = RigidBody::new(transform.next().unwrap());

        rb.set_moi_as_cuboid((1., 1., 1.).into());

        println!("WHATS THE VECTOR {:?}", rb.sqrt_angular_mass);

        assert_eq!(
            rb.angular_vel_per_impulse((1., 0., 0.).into(), (1., 0., 0., 0.).into()),
            2.
        );
    }
}
