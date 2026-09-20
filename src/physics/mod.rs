mod collider;
mod contact;
mod geo_alg;
// mod geo_alg_com;

use crate::{
    game_objects::{RunnableMut, transform::{Transform, TransformID, TransformSystem}}, utilities::math::skew,
};
use cgmath::{InnerSpace, Matrix, Matrix3, Matrix4, One, SquareMatrix, Vector3, Zero};
use collider::ContactIdPair;
pub use collider::{ColliderSystem, CuboidCollider, LeafInHierachy};
use std::{
    ops::ControlFlow, sync::{Arc, RwLock, atomic::AtomicUsize},
};

type Vector = Vector3<f32>;

const GRAVITY: Vector = Vector {
    x: 0.,
    y: -9.81,
    z: 0.,
};
/// Frames (updates) a rb must rest before beginning sleep
const SLEEP_TIMER: u8 = 10;
/// min velocity needed to reset sleep timer
const WAKE_VEL_SQR: f32 = 0.05;
/// min angular velocity needed to reset sleep timer
const WAKE_BIVEL_SQR: f32 = 0.05;

#[allow(dead_code)]
pub fn matrix_truncate(model: &Matrix4<f32>) -> Matrix3<f32> {
    Matrix3::from_cols(model.x.truncate(), model.y.truncate(), model.z.truncate())
}
/// Has to be attached to a root transform
pub struct RigidBody {
    /// Must be a root transform, is considered the centre of mass
    pub transform: TransformID,
    pub velocity: Vector,
    pub bivelocity: Vector,

    pub inv_mass: f32,
    /// sqrt of masses at unit distance on principle axes
    pub inv_moi: Matrix3<f32>,
    pub gravity_multiplier: f32,

    /// heap index of contacts this rb is a part of
    pub contact_refs: Vec<Arc<AtomicUsize>>,
    pub past_contacts: Vec<(u8, ContactIdPair)>,

    pub old_velocity: Vector,

    /// were contacts cached here last frame
    pub caching_contacts: bool,

    /// frames (updates) before rb starts sleeping, 0 means currently sleeping
    pub sleep_timer: u8,
}
impl RigidBody {
    /// Transform must be a root transform
    pub fn new(transform: TransformID) -> Self {
        RigidBody {
            transform,
            velocity: Vector::zero(),
            bivelocity: Vector::zero(),

            inv_mass: 1.,
            inv_moi: Matrix3::one(),
            gravity_multiplier: 1.,

            contact_refs: Vec::new(),
            past_contacts: Vec::new(),

            old_velocity: Vector::zero(),

            caching_contacts: false,

            sleep_timer: SLEEP_TIMER,
        }
    }

    pub fn update(&mut self, transform: &mut Transform, delta_secs: f32) {
        // no velocity updates if sleeping
        if self.is_awake() {
            if self.velocity.magnitude2() < WAKE_VEL_SQR
                && self.bivelocity.magnitude2() < WAKE_BIVEL_SQR
            {
                // println!("[Debug] RB ({:?}) sleep decrement", self.transform);
                self.sleep_timer -= 1;
            } else {
                self.wake();
            }

            self.velocity *= 1. - 0.05 * delta_secs;
            self.bivelocity *= 1. - 0.05 * delta_secs;

            self.velocity += GRAVITY * delta_secs * self.gravity_multiplier;

            transform.mutate(|t, r, _| {
                *t += self.velocity * delta_secs;
                *r = geo_alg::bivec_exp((delta_secs / 2.) * self.bivelocity).into_quaternion() * *r;
            });
        } else {
            self.velocity = Vector::zero();
            self.bivelocity = Vector::zero();
        }

        // println!(
        //     "[RB Post Update] ({:?})\n\tpos: {:?}\n\trot: {:?}\n\tvel: {:?}({:?})\n\tbiv: {:?}({:?})\n\tsleep_timer: {:?}",
        //     self.transform,
        //     transform.get_local_transform().translation,
        //     transform.get_local_transform().rotation,
        //     self.velocity,
        //     self.velocity.magnitude(),
        //     self.bivelocity,
        //     self.bivelocity.magnitude(),
        //     self.sleep_timer
        // );

        self.contact_refs.clear();
        if self.caching_contacts {
            self.caching_contacts = false;
        } else {
            self.past_contacts.clear();
        }

        // println!("MASSES: {:?}", self.sqrt_angular_mass);
    }

    /// reset internal sleep timer to wake rb
    pub fn wake(&mut self) {
        self.sleep_timer = SLEEP_TIMER;
    }

    pub fn is_awake(&self) -> bool {
        self.sleep_timer != 0
    }

    /// Apply impulse at a point relative to the target frame
    /// 
    /// impulse is in global space and rotation is the target's own rotation
    pub fn apply_impulse_rel(
        &mut self,
        rel_point: Vector,
        impulse: Vector,
        rotation: impl Into<Matrix3<f32>>,
    ) {
        self.velocity += impulse * self.inv_mass;

        let angular_inertia = self.w_per_i(rel_point, rotation.into());
        let delta_bv = angular_inertia * impulse;
        self.bivelocity += delta_bv;

        if self.velocity.magnitude2() >= WAKE_VEL_SQR
            || self.bivelocity.magnitude2() >= WAKE_BIVEL_SQR
        {
            self.wake();
        }

        // println!(
        //     "[Point impulse] ({:?})\n\timpulse: {:?}\n\tnew_v: {:?}\n\tnew_b: {:?}",
        //     self.transform, impulse, self.velocity, self.bivelocity,
        // );
    }

    /// Point and impulse are both in world space
    /// 
    /// Returns an error if the rb's transform cannot be found
    pub fn apply_impulse_global(
        &mut self,
        point: Vector,
        impulse: Vector,
        transforms: &mut TransformSystem,
    ) -> Result<(),()> {
        let transform_view = transforms.get_transform(&self.transform).ok_or(())?.get_local_transform();
        self.apply_impulse_rel(point - transform_view.translation, impulse, *transform_view.rotation);
        Ok(())
    }

    pub fn point_velocity(&self, rel_point: Vector) -> Vector {
        self.velocity + self.bivelocity.cross(rel_point)
    }

    /// Set principle axis masses assuming object is a cuboid of constant density, taking object scale into account
    ///
    /// Does nothing if inv_mass is zero (i.e infinite mass)
    pub fn set_moi_as_cuboid(&mut self, scale: Vector) {
        if self.inv_mass.is_zero() {
            return;
        }
        self.inv_moi = Matrix3::from_diagonal(scale.map(|c| 1. / (c * c)) * (self.inv_mass * 12.));
    }

    /// rotational acceleration per impulse at a point (does not include linear acceleration)
    /// 
    /// skew * rot * inv_moi * rot^T * -skew
    pub fn va_per_i(&self, rel_point: Vector, rotation: Matrix3<f32>) -> Matrix3<f32> {
        let point_squared = rel_point.magnitude2();
        if point_squared.is_zero() {
            return Matrix3::zero();
        }

        let t = skew(rel_point) * rotation;

        t * self.inv_moi * t.transpose()
    }

    /// angular velocity per impulse at a point
    /// 
    /// rot * inv_moi * rot^T * skew
    pub fn w_per_i(&self, rel_point: Vector, rotation: Matrix3<f32>) -> Matrix3<f32> {
        let point_squared = rel_point.magnitude2();
        if point_squared.is_zero() {
            return Matrix3::zero();
        }

        // let inv_moi = Matrix3::from_diagonal(self.moi.map(|c| 1. / c));

        // rot * inv_moi * rot^T * skew
        rotation * self.inv_moi * rotation.transpose() * skew(rel_point)
    }

    pub fn set_old_velocity(&mut self) {
        self.old_velocity = self.velocity;
    }

    /// search rb for matching contact id and remove it
    pub fn remove_cached_contact(&mut self, id: &ContactIdPair) {
        // let mut index = None;
        let index = self
            .past_contacts
            .iter()
            .enumerate()
            .try_for_each(|(i, item)| {
                if *id == item.1 {
                    // index = Some(i);
                    ControlFlow::Break(i)
                } else {
                    ControlFlow::Continue(())
                }
            });
        // for (i, item) in self.past_contacts.iter().enumerate() {
        //     if *id == item.1 {
        //         index = Some(i);
        //         break;
        //     }
        // }
        if let ControlFlow::Break(i) = index {
            self.past_contacts.remove(i);
        }
    }
}

impl RunnableMut for (&TransformID, &mut Arc<RwLock<RigidBody>>) {
    fn update(self, resources: &mut crate::game_objects::GameResources) {
        self.1.write().unwrap().update(
            resources.transforms.get_transform_mut(self.0).unwrap(),
            resources.last_delta_time,
        );
        // println!(
        //     "[RB] id: {:?}, model: {:?}",
        //     transfrom,
        //     self.transforms.get_global_model(transfrom)
        // );
    }
}
impl RunnableMut for &mut Arc<RwLock<RigidBody>> {
    fn update(self, _: &mut crate::game_objects::GameResources) {
        self.write().unwrap().set_old_velocity();
    }
}

#[cfg(test)]
mod physics_tests {
    use std::f32::consts::PI;

    use cgmath::{InnerSpace, Matrix3, Matrix4, One, Quaternion, Rad, SquareMatrix, Vector3};

    use crate::game_objects::transform::TransformSystem;
    use crate::physics::{RigidBody};

    #[test]
    fn quat_convert() {
        let quat = Quaternion::new((PI / 4.).cos(), (PI / 4.).sin(), 0., 0.);
        let mat: Matrix3<f32> = quat.into();
        println!("{:?}", quat * Vector3::new(0., 1., 0.));
        println!("{:?}", mat);
    }

    #[test]
    fn check_angular_vpi() {
        let mut transform = TransformSystem::new();
        let mut rb = RigidBody::new(transform.next().unwrap());
        rb.inv_mass = 0.5;

        rb.set_moi_as_cuboid((1., 1., 1.).into());

        println!("WHATS THE VECTOR {:?}", rb.inv_moi);

        // println!(
        //     "(1,0,0): {:?}",
        //     rb.angular_vel_per_impulse((1., 0., 0.).into(), (1., 0., 0., 0.).into())
        // );
        // println!(
        //     "(1,0,1): {:?}",
        //     rb.angular_vel_per_impulse((1., 0., 1.).into(), (1., 0., 0., 0.).into())
        // );
        // println!(
        //     "(1,0,-1): {:?}",
        //     rb.angular_vel_per_impulse((1., 0., -1.).into(), (1., 0., 0., 0.).into())
        // );
        // println!(
        //     "(-1,0,1): {:?}",
        //     rb.angular_vel_per_impulse((-1., 0., 1.).into(), (1., 0., 0., 0.).into())
        // );
        // println!(
        //     "(-1,0,-1): {:?}",
        //     rb.angular_vel_per_impulse((-1., 0., -1.).into(), (1., 0., 0., 0.).into())
        // );

        println!(
            "(1,0,0): {:?}",
            rb.va_per_i((0., 1., 0.).into(), Matrix3::one())
                * Vector3 {
                    x: 0.,
                    y: 0.,
                    z: 1.
                }
        );
        println!(
            "(1,0,1): {:?}",
            rb.va_per_i((1., 0., -1.).into(), Matrix3::one())
                * Vector3 {
                    x: 0.,
                    y: 1.,
                    z: 0.
                }
        );

        println!(
            "(1,0,0): {:?}",
            rb.w_per_i((0., 1., 0.).into(), Matrix3::one())
                * Vector3 {
                    x: 0.,
                    y: 0.,
                    z: 1.
                }
        );
    }

    #[test]
    fn quick_inv() {
        let rot = Matrix4::from_axis_angle(Vector3 { x: 1., y: 1., z: 0. }.normalize(), Rad(1.2));
        let scale = Matrix4::from_nonuniform_scale(0.2, 0.4, 2.);
        let translate = Matrix4::from_translation((10., -2., -5.).into());

        let model = translate * scale * rot;
        let inv = model.invert().unwrap();

        println!("Model: {:?}", model);
        println!("Inv: {:?}", inv);
        println!("One?: {:?}", inv * model);
        assert!((inv * model).is_one());
    }
}
