use super::{geo_alg::bivec_exp, RigidBody, Vector};
use crate::{game_objects::transform::TransformSystem, utilities::MaxHeap};
use cgmath::InnerSpace;
use std::sync::{atomic::AtomicUsize, Arc, RwLock};

const PEN_RESTITUTION: f32 = 1.000001;

#[derive(PartialEq, Clone, Copy)]
struct OrdF32(pub f32);

pub struct ContactResolver {
    pending_contacts: MaxHeap<OrdF32, Contact>,
    settled_contacts: Vec<(Arc<AtomicUsize>, Contact)>,
}

pub struct Contact {
    position: Vector,
    /// points from rb_1 to rb_2
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

    linear_inertia: f32,
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

    pub fn get_contacts(&self) -> impl Iterator<Item = &Contact> {
        self.pending_contacts.iter()
    }

    pub fn resolve(&mut self, transform_system: &mut TransformSystem) {
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

        self.clear();
    }

    fn resolve_penetration(&mut self, transform_system: &mut TransformSystem) {
        while let Some((index, contact)) = self.pending_contacts.extract_min() {
            if let Some(rb_2) = &contact.rb_2 {
                // calculate move
                let inv_total_inertia = 1.
                    / (contact.rb_1.linear_inertia
                        + contact.rb_1.angular_inertia
                        + rb_2.linear_inertia
                        + rb_2.angular_inertia);

                contact.rb_1.resolve_penetration(
                    -contact.normal,
                    contact.penetration,
                    inv_total_inertia,
                    &mut self.pending_contacts,
                    transform_system,
                );
                rb_2.resolve_penetration(
                    contact.normal,
                    contact.penetration,
                    inv_total_inertia,
                    &mut self.pending_contacts,
                    transform_system,
                );
            } else {
                // calculate move
                let inv_total_inertia =
                    1. / (contact.rb_1.linear_inertia + contact.rb_1.angular_inertia);

                contact.rb_1.resolve_penetration(
                    -contact.normal,
                    contact.penetration,
                    inv_total_inertia,
                    &mut self.pending_contacts,
                    transform_system,
                );
            }

            self.settled_contacts.push((index, contact));
        }
    }
    fn resolve_velocity(&mut self, transform_system: &TransformSystem) {}

    // drop all contacts from pending and settled and remove their reference from their rigidbodies
    pub fn clear(&mut self) {
        while let Some(contact) = self.pending_contacts.extract_min() {
            self.settled_contacts.push(contact);
        }

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

        println!(
            "[Contact creation] pos: {:?}, normal: {:?}, pen: {:?}",
            position, normal, penetration
        );

        // aquire rb 1 data
        let mut rb_guard_1 = rb_1.write().unwrap();
        let transform_1 = transform_sys
            .get_transform(&rb_guard_1.transform)
            .unwrap()
            .get_local_transform();

        let relative_pos = position - transform_1.translation;
        let point_vel_1 = rb_guard_1.point_velocity(relative_pos);

        // normal points away from contact point 1 (assuming convex shape)
        let normal = relative_pos.dot(normal).signum() * normal;

        let linear_inertia = rb_guard_1.inv_mass;
        // n x r
        let torque_per_impulse = -normal.cross(relative_pos);
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
            linear_inertia,
            angular_inertia,
        };

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
            let torque_per_impulse = normal.cross(relative_pos);
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
                linear_inertia: inv_mass,
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

    /// returns (position, normal, penetration)
    pub fn get_debug_info(&self) -> (Vector, Vector, f32) {
        (self.position, self.normal, self.penetration)
    }
}

impl RigidBodyRef {
    /// normal is in target move direction to resolve penetration
    fn resolve_penetration(
        &self,
        normal: Vector,
        penetration: f32,
        inv_total_inertia: f32,
        pending_contacts: &mut MaxHeap<OrdF32, Contact>,
        transform_system: &mut TransformSystem,
    ) {
        // calculate move
        let linear_move = PEN_RESTITUTION * penetration * self.linear_inertia * inv_total_inertia;
        let angular_move = PEN_RESTITUTION * penetration * self.angular_inertia * inv_total_inertia;

        let move_1 = linear_move * normal;
        let rotate_1 = angular_move * self.torque_per_impulse / self.relative_pos.magnitude2();

        // apply move
        let guard_1 = self.rigidbody.read().unwrap();
        transform_system
            .get_transform_mut(&guard_1.transform)
            .unwrap()
            .mutate(|translation, rotation, _| {
                *translation += move_1;
                *rotation = bivec_exp(rotate_1 * 0.5).into_quaternion() * *rotation;
            });

        // update penetration of contacts on the same rb
        for (i, other_index) in guard_1.contact_refs.iter().enumerate() {
            // could compare heap index Arc instead
            if i == self.index {
                // skip self
                continue;
            }

            let other_index_loaded = other_index.load(std::sync::atomic::Ordering::Acquire);
            if other_index_loaded < pending_contacts.len() {
                pending_contacts.modify_key(other_index_loaded, |other_contact| {
                    let norm_mult = if Arc::ptr_eq(&self.rigidbody, &self.rigidbody) {
                        -1.
                    } else {
                        1.
                    };

                    other_contact.penetration += norm_mult * move_1.dot(other_contact.normal);
                    other_contact.penetration +=
                        norm_mult * rotate_1.dot(other_contact.rb_1.torque_per_impulse);

                    other_contact.penetration.into()
                });
            }
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
