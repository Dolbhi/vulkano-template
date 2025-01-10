mod collider;
mod contact;
mod geo_alg;
// mod geo_alg_com;

use crate::game_objects::transform::{Transform, TransformID};
use cgmath::{InnerSpace, Matrix, Matrix3, Matrix4, Quaternion, Vector3, Zero};
pub use collider::{ColliderSystem, CuboidCollider, LeafInHierachy};
use std::sync::{atomic::AtomicUsize, Arc};

type Vector = Vector3<f32>;

const GRAVITY: Vector = Vector {
    x: 0.,
    y: -9.81,
    z: 0.,
};

/// Invert a othonormal model matrix that has no skew
///
/// Assumes matrix is normalised (model.w.w == 1)
#[allow(dead_code)]
pub fn quick_inverse(model: &mut Matrix4<f32>) {
    // reverse translation
    model.w *= -1.;
    model.w.w = 1.;

    // reverse rotation and scale
    for i in [0, 1, 2] {
        let m_2 = model[i].magnitude2();
        model[i] /= m_2;
    }
    model.swap_elements((0, 1), (1, 0));
    model.swap_elements((1, 2), (2, 1));
    model.swap_elements((2, 0), (0, 2));
}

#[allow(dead_code)]
pub fn matrix_truncate(model: &Matrix4<f32>) -> Matrix3<f32> {
    Matrix3::from_cols(model.x.truncate(), model.y.truncate(), model.z.truncate())
}

pub struct RigidBody {
    pub transform: TransformID,
    pub velocity: Vector,
    pub bivelocity: Vector,

    pub inv_mass: f32,
    /// sqrt of masses at unit distance on principle axes
    pub principle_moi: Vector,
    pub gravity_multiplier: f32,

    pub contact_refs: Vec<Arc<AtomicUsize>>,

    pub old_velocity: Vector,
}
impl RigidBody {
    pub fn new(transform: TransformID) -> Self {
        RigidBody {
            transform,
            velocity: Vector::zero(),
            bivelocity: Vector::zero(),

            inv_mass: 1.,
            principle_moi: (1., 1., 1.).into(),
            gravity_multiplier: 1.,

            contact_refs: Vec::new(),

            old_velocity: Vector::zero(),
        }
    }

    pub fn update(&mut self, transform: &mut Transform, delta_secs: f32) {
        // self.velocity *= 1. - 0.05 * delta_secs;
        // self.bivelocity *= 1. - 0.05 * delta_secs;

        self.velocity += GRAVITY * delta_secs * self.gravity_multiplier;

        transform.mutate(|t, r, _| {
            *t += self.velocity * delta_secs;
            *r = geo_alg::bivec_exp((delta_secs / 2.) * self.bivelocity).into_quaternion() * *r;
        });

        // println!("MASSES: {:?}", self.sqrt_angular_mass);
    }

    pub fn apply_impulse(&mut self, point: Vector, impulse: Vector, rotation: Quaternion<f32>) {
        self.velocity += impulse * self.inv_mass;

        // let impulse_mag = impulse.magnitude();
        // let torque_per_impulse = impulse.cross(point) / impulse_mag;
        // let angular_inertia = self.angular_vel_per_impulse(torque_per_impulse, rotation);
        // self.bivelocity += impulse_mag * torque_per_impulse * angular_inertia;

        let torque = -impulse.cross(point);
        let angular_inertia = self.angular_vel_per_impulse(torque.normalize(), rotation);
        self.bivelocity += torque * angular_inertia;

        println!(
            "[Point impulse] point: {:?}, impulse: {:?}, delta_v: {:?}, angular_inertia: {:?}",
            point,
            impulse,
            impulse * self.inv_mass,
            angular_inertia
        );
    }

    pub fn point_velocity(&self, point: Vector) -> Vector {
        self.velocity + self.bivelocity.cross(point)
    }

    /// Set principle axis masses assuming object is a cuboid of constant density, taking object scale into account
    ///
    /// Does nothing if inv_mass is zero (i.e infinite mass)
    pub fn set_moi_as_cuboid(&mut self, scale: Vector) {
        if self.inv_mass.is_zero() {
            return;
        }
        self.principle_moi = scale.map(|c| c * c) / (self.inv_mass * 12.);
    }

    /// inverse moment of inertia about an axis (and other stuff), calculated via black magic
    pub fn angular_vel_per_impulse(
        &self,
        torque_per_impulse: Vector,
        rotation: Quaternion<f32>,
    ) -> f32 {
        let tpi_squared = torque_per_impulse.magnitude2();
        if tpi_squared.is_zero() {
            return 0.;
        }

        // let world_sam = rotation * self.sqrt_angular_mass;
        // (tpi_squared * tpi_squared) / torque_per_impulse.cross(world_sam).magnitude2()
        let local_torque = rotation.conjugate() * torque_per_impulse;
        let moi = local_torque.dot(
            (
                local_torque[0] * (self.principle_moi[0]),
                local_torque[1] * (self.principle_moi[1]),
                local_torque[2] * (self.principle_moi[2]),
            )
                .into(),
        );
        (tpi_squared * tpi_squared) / moi
        // 6.
    }

    pub fn set_old_velocity(&mut self) {
        self.old_velocity = self.velocity;
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
        rb.inv_mass = 0.5;

        rb.set_moi_as_cuboid((1., 1., 1.).into());

        println!("WHATS THE VECTOR {:?}", rb.principle_moi);

        println!(
            "(1,0,0): {:?}",
            rb.angular_vel_per_impulse((1., 0., 0.).into(), (1., 0., 0., 0.).into())
        );
        println!(
            "(1,0,1): {:?}",
            rb.angular_vel_per_impulse((1., 0., 1.).into(), (1., 0., 0., 0.).into())
        );
        println!(
            "(1,0,-1): {:?}",
            rb.angular_vel_per_impulse((1., 0., -1.).into(), (1., 0., 0., 0.).into())
        );
        println!(
            "(-1,0,1): {:?}",
            rb.angular_vel_per_impulse((-1., 0., 1.).into(), (1., 0., 0., 0.).into())
        );
        println!(
            "(-1,0,-1): {:?}",
            rb.angular_vel_per_impulse((-1., 0., -1.).into(), (1., 0., 0., 0.).into())
        );
    }
}
