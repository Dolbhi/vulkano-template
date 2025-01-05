use std::sync::{atomic::AtomicUsize, Arc, RwLock};

use cgmath::{InnerSpace, Zero};

use crate::{
    game_objects::transform::{self, TransformSystem},
    utilities::MaxHeap,
};

use super::{RigidBody, Vector};

#[derive(PartialEq, Clone, Copy)]
struct OrdF32(pub f32);

pub struct ContactResolver {
    pending_contacts: MaxHeap<OrdF32, Contact>,
    settled_contacts: Vec<(Arc<AtomicUsize>, Contact)>,
}

pub struct Contact {
    position: Vector,
    normal: Vector,
    penetration: f32,

    rb_1: RigidBodyRef,
    rb_2: Option<RigidBodyRef>,

    closing_velocity: Vector,
    target_delta_velocity: f32,
}

struct RigidBodyRef {
    rigidbody: Arc<RwLock<RigidBody>>,
    /// location of this contact in rigidbody.contact_refs
    index: usize,

    relative_pos: Vector,
    torque_per_impulse: Vector,

    inv_mass: f32,
    angular_inertia: f32,
}

impl ContactResolver {
    pub fn new() -> Self {
        Self {
            pending_contacts: MaxHeap::new(),
            settled_contacts: Vec::new(),
        }
    }

    pub fn add_contact(&mut self, index: Arc<AtomicUsize>, contact: Contact) {
        self.pending_contacts
            .insert_with_ref(contact.penetration.into(), contact, index);
    }

    pub fn resolve(&mut self, transform_system: &TransformSystem) {
        self.resolve_penetration(transform_system);

        // re-insert contacts with velocity as value
        for (index, contact) in self.settled_contacts.drain(0..self.settled_contacts.len()) {
            self.pending_contacts.insert_with_ref(
                contact.target_delta_velocity.into(),
                contact,
                index,
            );
        }

        self.resolve_velocity(transform_system);

        // drop all lingering references to contacts in rigidbodies
        for (index, contact) in self.settled_contacts.drain(0..self.settled_contacts.len()) {
            if Arc::strong_count(&index) > 1 {
                contact.rb_1.rigidbody.write().unwrap().contact_refs.clear();

                if let Some(rb_2) = contact.rb_2 {
                    rb_2.rigidbody.write().unwrap().contact_refs.clear();
                }
            }
        }
    }

    fn resolve_penetration(&mut self, transform_system: &TransformSystem) {
        while let Some((index, contact)) = self.pending_contacts.extract_min() {
            // resolve penetration
            // update penetration of contacts on the same rb
            self.settled_contacts.push((index, contact));
        }
    }
    fn resolve_velocity(&mut self, transform_system: &TransformSystem) {}
}

impl Contact {
    /// create new contact, automatically adding itself to the respective rigidbodies' contact_refs
    pub fn new(
        transform_sys: &TransformSystem,
        position: Vector,
        normal: Vector,
        penetration: f32,
        rb_1: Arc<RwLock<RigidBody>>,
        rb_2: Option<Arc<RwLock<RigidBody>>>,
    ) -> (Arc<AtomicUsize>, Self) {
        let heap_index = Arc::new(AtomicUsize::new(0));

        // aquire rb 1 data
        let mut rb_guard_1 = rb_1.write().unwrap();
        let transform_1 = transform_sys
            .get_transform(&rb_guard_1.transform)
            .unwrap()
            .get_local_transform();

        let relative_pos = position - transform_1.translation;
        let point_vel_1 = rb_guard_1.point_velocity(relative_pos);

        // normal points away from contact point 1
        let normal = point_vel_1.dot(normal).signum() * normal;

        let inv_mass = rb_guard_1.inv_mass;
        // n x r
        let torque_per_impulse = normal.cross(relative_pos);
        let angular_inertia =
            rb_guard_1.angular_vel_per_impulse(torque_per_impulse, *transform_1.rotation);

        // link to rb
        let index = rb_guard_1.contact_refs.len();
        rb_guard_1.contact_refs.push(heap_index.clone());
        drop(rb_guard_1);
        let rb_1 = RigidBodyRef {
            rigidbody: rb_1,
            index,
            relative_pos,
            torque_per_impulse,
            inv_mass,
            angular_inertia,
        };

        // rb_lock_1
        if let Some(rb_2) = rb_2 {
            // aquire rb 1 data
            let mut rb_guard_2 = rb_2.write().unwrap();
            let transform_2 = transform_sys
                .get_transform(&rb_guard_2.transform)
                .unwrap()
                .get_local_transform();
            let relative_pos = position - transform_2.translation;
            let point_vel_2 = rb_guard_2.point_velocity(relative_pos);

            let inv_mass = rb_guard_2.inv_mass;
            // n x r
            let torque_per_impulse = -normal.cross(relative_pos);
            let angular_inertia =
                rb_guard_2.angular_vel_per_impulse(torque_per_impulse, *transform_2.rotation);

            // link to rb
            let index = rb_guard_2.contact_refs.len();
            rb_guard_2.contact_refs.push(heap_index.clone());
            drop(rb_guard_2);
            let rb_2 = RigidBodyRef {
                rigidbody: rb_2,
                index,
                relative_pos,
                torque_per_impulse,
                inv_mass,
                angular_inertia,
            };

            let closing_velocity = point_vel_1 - point_vel_2;
            let target_delta_velocity = -2. * closing_velocity.dot(normal);

            (
                heap_index,
                Contact {
                    position,
                    normal,
                    penetration,

                    rb_1,
                    rb_2: Some(rb_2),

                    closing_velocity,
                    target_delta_velocity,
                },
            )
        } else {
            (
                heap_index,
                Contact {
                    position,
                    normal,
                    penetration,

                    rb_1,
                    rb_2: None,

                    closing_velocity: point_vel_1,
                    target_delta_velocity: -2. * point_vel_1.dot(normal),
                },
            )
        }
    }
}

impl Eq for OrdF32 {}
impl PartialOrd for OrdF32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrdF32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
impl From<f32> for OrdF32 {
    fn from(value: f32) -> Self {
        OrdF32(value)
    }
}
impl From<OrdF32> for f32 {
    fn from(value: OrdF32) -> Self {
        value.0
    }
}
