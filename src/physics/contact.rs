use super::{collider::ContactIdPair, geo_alg::bivec_exp, RigidBody, Vector};
use crate::{game_objects::transform::TransformSystem, utilities::MaxHeap};
use cgmath::InnerSpace;
use std::sync::{atomic::AtomicUsize, Arc, RwLock};

const PEN_RESTITUTION: f32 = 1.;
const MIN_BOUNCE_VEL: f32 = 0.2;
const ANGULAR_MOVE_LIMIT_RAD: f32 = 0.5;
const MAX_CONTACT_AGE: u8 = 4;
const VELOCITY_ITER_LIMIT: u32 = 100;

#[derive(PartialEq, Clone, Copy)]
struct OrdF32(pub f32);

pub struct ContactResolver {
    pending_contacts: MaxHeap<OrdF32, Contact>,
    settled_contacts: Vec<(Arc<AtomicUsize>, Contact)>,

    pub past_contacts: Vec<(Vector, Vector, u8)>,
}

pub struct Contact {
    position: Vector,
    /// points from rb_1 to rb_2
    normal: Vector,
    penetration: f32,

    rb_1: RigidBodyRef,
    rb_2: Option<RigidBodyRef>,

    target_delta_velocity: f32,

    contact_id: ContactIdPair,

    age: u8,
}

struct RigidBodyRef {
    rigidbody: Arc<RwLock<RigidBody>>,
    /// location of this contact in rigidbody.contact_refs
    index: usize,

    relative_pos: Vector,
    point_vel: Vector,
    torque_per_impulse: Vector,

    linear_inertia: f32,
    angular_inertia: f32,
}

impl ContactResolver {
    pub fn new() -> Self {
        Self {
            pending_contacts: MaxHeap::new(),
            settled_contacts: Vec::new(),
            past_contacts: Vec::new(),
        }
    }

    pub fn add_contact(&mut self, index: Arc<AtomicUsize>, contact: Contact) {
        self.pending_contacts
            .insert_with_ref(contact.penetration.into(), contact, index);
    }

    pub fn get_contacts(&self) -> impl ExactSizeIterator<Item = &Contact> {
        self.pending_contacts.iter()
    }

    pub fn resolve(&mut self, transform_system: &mut TransformSystem) {
        println!("-----Resolve Start-----");
        self.resolve_penetration(transform_system);

        // re-insert contacts with velocity as value
        for (index, contact) in self.settled_contacts.drain(..) {
            self.pending_contacts.insert_with_ref(
                contact.target_delta_velocity.into(),
                contact,
                index,
            );
        }

        self.resolve_velocity();

        self.clear();
    }

    fn resolve_penetration(&mut self, transform_system: &mut TransformSystem) {
        while let Some((index, contact)) = self.pending_contacts.extract_min() {
            println!(
                "[Penetration resolution start] pos: {:?}, normal: {:?}, pen: {:?}, age: {:?}",
                contact.position, contact.normal, contact.penetration, contact.age
            );

            println!(
                "\t[rb1] rel_pos: {:?}, t_per_i: {:?}, l_inertia: {:?}, a_inertia: {:?}",
                contact.rb_1.relative_pos,
                contact.rb_1.torque_per_impulse,
                contact.rb_1.linear_inertia,
                contact.rb_1.angular_inertia,
            );

            if let Some(rb_2) = &contact.rb_2 {
                println!(
                    "\t[rb2] rel_pos: {:?}, t_per_i: {:?}, l_inertia: {:?}, a_inertia: {:?}",
                    rb_2.relative_pos,
                    rb_2.torque_per_impulse,
                    rb_2.linear_inertia,
                    rb_2.angular_inertia,
                );

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
    fn resolve_velocity(&mut self) {
        let mut iters = 0;
        while let Some((index, mut contact)) = self.pending_contacts.extract_min() {
            if iters > VELOCITY_ITER_LIMIT || contact.target_delta_velocity <= 0.001 {
                self.settled_contacts.push((index, contact));
                break;
            }
            iters += 1;

            // println!(
            //     "[Velocity resolution start] pos: {:?}, normal: {:?}, vel: {:?}, age: {:?}",
            //     contact.position, contact.normal, contact.target_delta_velocity, contact.age
            // );

            // println!(
            //     "\t[rb1] point_vel: {:?}, t_per_i: {:?}, l_inertia: {:?}, a_inertia: {:?}",
            //     contact.rb_1.point_vel,
            //     contact.rb_1.torque_per_impulse,
            //     contact.rb_1.linear_inertia,
            //     contact.rb_1.angular_inertia,
            // );

            if let Some(rb_2) = &contact.rb_2 {
                // println!(
                //     "\t[rb2] point_vel: {:?}, t_per_i: {:?}, l_inertia: {:?}, a_inertia: {:?}",
                //     rb_2.point_vel,
                //     rb_2.torque_per_impulse,
                //     rb_2.linear_inertia,
                //     rb_2.angular_inertia,
                // );

                // if contact.target_delta_velocity < 0.01 {

                // }

                // calculate inertia
                let inv_total_inertia = 1.
                    / (contact.rb_1.linear_inertia
                        + contact.rb_1.angular_inertia
                        + rb_2.linear_inertia
                        + rb_2.angular_inertia);

                contact.rb_1.resolve_velocity(
                    -contact.normal,
                    contact.target_delta_velocity,
                    inv_total_inertia,
                    &mut self.pending_contacts,
                );
                rb_2.resolve_velocity(
                    contact.normal,
                    contact.target_delta_velocity,
                    inv_total_inertia,
                    &mut self.pending_contacts,
                );
            } else {
                // calculate move
                let inv_total_inertia =
                    1. / (contact.rb_1.linear_inertia + contact.rb_1.angular_inertia);

                contact.rb_1.resolve_velocity(
                    -contact.normal,
                    contact.target_delta_velocity,
                    inv_total_inertia,
                    &mut self.pending_contacts,
                );
            }

            // self.settled_contacts.push((index, contact));

            let guard_1 = contact.rb_1.rigidbody.read().unwrap();
            let old_rel_vel = contact.rb_1.point_vel;
            let new_rel_vel = guard_1.point_velocity(contact.rb_1.relative_pos);
            contact.rb_1.point_vel = new_rel_vel;
            contact.target_delta_velocity += contact.normal.dot(new_rel_vel - old_rel_vel);
            drop(guard_1);

            if let Some(rb_2) = &mut contact.rb_2 {
                let guard_2 = rb_2.rigidbody.read().unwrap();
                let old_rel_vel = rb_2.point_vel;
                let new_rel_vel = guard_2.point_velocity(rb_2.relative_pos);
                rb_2.point_vel = new_rel_vel;
                contact.target_delta_velocity -= contact.normal.dot(new_rel_vel - old_rel_vel);
            }

            self.pending_contacts.insert_with_ref(
                contact.target_delta_velocity.into(),
                contact,
                index,
            );
        }
    }

    /// drop all contacts from pending and settled and push their ids to past contacts cache
    pub fn clear(&mut self) {
        while let Some(contact) = self.pending_contacts.extract_min() {
            self.settled_contacts.push(contact);
        }

        self.past_contacts.clear();

        // add to past contacts
        for (_, contact) in self.settled_contacts.drain(..) {
            self.past_contacts
                .push((contact.position, contact.normal, contact.age));

            let mut rb_1 = contact.rb_1.rigidbody.write().unwrap();
            if contact.age + 1 < MAX_CONTACT_AGE {
                rb_1.past_contacts
                    .push((contact.age + 1, contact.contact_id));
                rb_1.caching_contacts = true;
            }
        }
    }
}
impl Default for ContactResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Contact {
    /// create new contact, automatically adding itself to the respective rigidbodies' contact_refs
    ///
    /// normal should be normalised but either pointing towards or away from 1 is fine.
    pub fn new(
        transform_sys: &TransformSystem,
        position: Vector,
        normal: Vector,
        penetration: f32,
        // rb_1: Arc<RwLock<RigidBody>>,
        // rb_2: Option<Arc<RwLock<RigidBody>>>,
        contact_id: ContactIdPair,
        age: u8,
    ) -> (Arc<AtomicUsize>, Self) {
        let rb_1 = contact_id
            .0
            .collider
            .upgrade()
            .unwrap()
            .get_rigidbody()
            .as_ref()
            .unwrap()
            .clone();
        let rb_2 = contact_id
            .1
            .collider
            .upgrade()
            .unwrap()
            .get_rigidbody()
            .clone();

        let heap_index = Arc::new(AtomicUsize::new(usize::MAX));

        // aquire rb 1 data
        let mut rb_guard_1 = rb_1.write().unwrap();
        let transform_1 = transform_sys
            .get_transform(&rb_guard_1.transform)
            .unwrap()
            .get_local_transform();

        let relative_pos = position - transform_1.translation;
        let point_vel_1 = rb_guard_1.point_velocity(relative_pos);
        let old_vel_1 = rb_guard_1.old_velocity;

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
        if age == 0 {
            rb_guard_1.remove_cached_contact(&contact_id);
        }
        // search rb for matching contact id
        drop(rb_guard_1);
        let rb_1 = RigidBodyRef {
            rigidbody: rb_1,
            index,
            relative_pos,
            point_vel: point_vel_1,
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
            let old_vel_2 = rb_guard_2.old_velocity;

            let inv_mass = rb_guard_2.inv_mass;
            // n x r
            let torque_per_impulse = normal.cross(relative_pos);
            let angular_inertia =
                rb_guard_2.angular_vel_per_impulse(torque_per_impulse, *transform_2.rotation);

            // link to rb
            let index = rb_guard_2.contact_refs.len();
            rb_guard_2.contact_refs.push(heap_index.clone());
            if age == 0 {
                rb_guard_2.remove_cached_contact(&contact_id);
            }
            drop(rb_guard_2);
            let rb_2 = RigidBodyRef {
                rigidbody: rb_2,
                index,
                relative_pos,
                point_vel: point_vel_2,
                torque_per_impulse,
                linear_inertia: inv_mass,
                angular_inertia,
            };

            let closing_velocity = point_vel_1 - point_vel_2;
            let old_closing_velocity = old_vel_1 - old_vel_2;
            let restituition = if -closing_velocity.dot(normal) < MIN_BOUNCE_VEL {
                0.0
            } else {
                0.5
            };
            let target_delta_velocity =
                (closing_velocity + restituition * old_closing_velocity).dot(normal);
            //   ^cancels out the current velocity
            //                      ^bounce using only old velocity

            (
                heap_index,
                Contact {
                    position,
                    normal,
                    penetration,

                    rb_1,
                    rb_2: Some(rb_2),

                    target_delta_velocity,

                    contact_id,
                    age,
                },
            )
        } else {
            let restituition = if -point_vel_1.dot(normal) < MIN_BOUNCE_VEL {
                0.0
            } else {
                0.5
            };
            (
                heap_index,
                Contact {
                    position,
                    normal,
                    penetration,

                    rb_1,
                    rb_2: None,

                    target_delta_velocity: (point_vel_1 + restituition * old_vel_1).dot(normal),

                    contact_id,
                    age,
                },
            )
        }
    }

    /// returns (position, normal, penetration)
    pub fn get_debug_info(&self) -> (Vector, Vector, f32) {
        (self.position, self.normal, self.penetration)
    }

    pub fn get_rigidbodies(&self) -> (&Arc<RwLock<RigidBody>>, Option<&Arc<RwLock<RigidBody>>>) {
        (
            &self.rb_1.rigidbody,
            self.rb_2.as_ref().map(|r| &r.rigidbody),
        )
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
        // no resolution needed if penetration <= 0
        if penetration <= 0. {
            return;
        }

        // calculate move
        let mut linear_move =
            PEN_RESTITUTION * penetration * self.linear_inertia * inv_total_inertia;
        let mut angular_move =
            PEN_RESTITUTION * penetration * self.angular_inertia * inv_total_inertia;

        let angular_move_rad = angular_move / self.relative_pos.magnitude();
        if angular_move_rad > ANGULAR_MOVE_LIMIT_RAD {
            let excess = angular_move * (1.0 - (ANGULAR_MOVE_LIMIT_RAD / angular_move_rad));
            println!(
                "\t[Penetration resolution] rotation limit hit! Excess: {:?}",
                excess
            );
            angular_move -= excess;
            linear_move += excess;
        }

        let linear_move = linear_move * normal;
        let rotate = -angular_move * self.torque_per_impulse / self.relative_pos.magnitude2();

        println!(
            "\t[Penetration resolution] move: {:?}, rotate: {:?}",
            linear_move, rotate
        );

        // apply move
        let guard_1 = self.rigidbody.read().unwrap();
        transform_system
            .get_transform_mut(&guard_1.transform)
            .unwrap()
            .mutate(|translation, rotation, _| {
                *translation += linear_move;
                *rotation = bivec_exp(rotate * 0.5).into_quaternion() * *rotation;
            });

        // update penetration of contacts on the same rb
        for (i, other_index) in guard_1.contact_refs.iter().enumerate() {
            // could compare heap index Arc instead
            if i == self.index {
                // skip self
                continue;
            }

            let other_index_loaded = other_index.load(std::sync::atomic::Ordering::Acquire);
            if other_index_loaded != usize::MAX {
                //< pending_contacts.len() {
                pending_contacts.modify_key(other_index_loaded, |other_contact| {
                    let (norm_mult, other_rb) =
                        if Arc::ptr_eq(&self.rigidbody, &other_contact.rb_1.rigidbody) {
                            (1., &other_contact.rb_1)
                        } else {
                            (-1., other_contact.rb_2.as_ref().unwrap()) // please
                        };

                    other_contact.penetration += norm_mult * linear_move.dot(other_contact.normal);
                    other_contact.penetration += rotate.dot(other_rb.torque_per_impulse);

                    other_contact.penetration.into()
                });
            }
        }
    }

    /// normal is in target move direction to resolve velocity
    fn resolve_velocity(
        &self,
        normal: Vector,
        target_delta_v: f32,
        inv_total_inertia: f32,
        pending_contacts: &mut MaxHeap<OrdF32, Contact>,
    ) {
        // no resolution needed if target_delta_v <= 0
        if target_delta_v <= 0. {
            return;
        }

        // calculate move
        let linear_accel = target_delta_v * self.linear_inertia * inv_total_inertia;
        let angular_accel = -target_delta_v * self.angular_inertia * inv_total_inertia;

        let linear_accel = linear_accel * normal;
        let angular_accel =
            angular_accel * self.torque_per_impulse / self.relative_pos.magnitude2();

        // println!(
        //     "\t[Velocity resolution] linear: {:?}, angular: {:?}",
        //     linear_accel, angular_accel
        // );

        // apply move
        let mut guard_1 = self.rigidbody.write().unwrap();
        guard_1.velocity += linear_accel;
        guard_1.bivelocity += angular_accel;

        // println!(
        //     "\t[Velocity results] linear: {:?}, angular: {:?}",
        //     guard_1.velocity, guard_1.bivelocity
        // );

        // update penetration of contacts on the same rb
        for (i, other_index) in guard_1.contact_refs.iter().enumerate() {
            // could compare heap index Arc instead
            if i == self.index {
                // skip self
                continue;
            }

            let other_index_loaded = other_index.load(std::sync::atomic::Ordering::Acquire);
            if other_index_loaded != usize::MAX {
                //< pending_contacts.len() {
                pending_contacts.modify_key(other_index_loaded, |other_contact| {
                    let (norm_mult, other_rb) =
                        if Arc::ptr_eq(&self.rigidbody, &other_contact.rb_1.rigidbody) {
                            (1., &mut other_contact.rb_1)
                        } else {
                            (-1., other_contact.rb_2.as_mut().unwrap()) // please
                        };

                    let old_rel_vel = other_rb.point_vel;
                    let new_rel_vel = guard_1.point_velocity(other_rb.relative_pos);
                    other_rb.point_vel = new_rel_vel;

                    other_contact.target_delta_velocity +=
                        norm_mult * other_contact.normal.dot(new_rel_vel - old_rel_vel);

                    other_contact.target_delta_velocity.into()
                });
            }
        }
    }

    // fn negate_velocity(&self, normal: Vector, other_vel: Vector, pending_contacts: &mut MaxHeap<OrdF32, Contact>) {
    //     let linear_accel = self.point_vel * normal;
    //     let angular_accel =
    //         angular_accel * self.torque_per_impulse / self.relative_pos.magnitude2();

    //     println!(
    //         "\t[Velocity resolution] linear: {:?}, angular: {:?}",
    //         linear_accel, angular_accel
    //     );

    //     // apply move
    //     let mut guard_1 = self.rigidbody.write().unwrap();
    //     guard_1.velocity += linear_accel;
    //     guard_1.bivelocity += angular_accel;

    //     println!(
    //         "\t[Velocity results] linear: {:?}, angular: {:?}",
    //         guard_1.velocity, guard_1.bivelocity
    //     );

    //     // update penetration of contacts on the same rb
    //     for (i, other_index) in guard_1.contact_refs.iter().enumerate() {
    //         // could compare heap index Arc instead
    //         if i == self.index {
    //             // skip self
    //             continue;
    //         }

    //         let other_index_loaded = other_index.load(std::sync::atomic::Ordering::Acquire);
    //         if other_index_loaded != usize::MAX {
    //             //< pending_contacts.len() {
    //             pending_contacts.modify_key(other_index_loaded, |other_contact| {
    //                 let (norm_mult, other_rb) =
    //                     if Arc::ptr_eq(&self.rigidbody, &other_contact.rb_1.rigidbody) {
    //                         (1., &mut other_contact.rb_1)
    //                     } else {
    //                         (-1., other_contact.rb_2.as_mut().unwrap()) // please
    //                     };

    //                 let old_rel_vel = other_rb.point_vel;
    //                 let new_rel_vel = guard_1.point_velocity(other_rb.relative_pos);
    //                 other_rb.point_vel = new_rel_vel;

    //                 other_contact.target_delta_velocity +=
    //                     norm_mult * other_contact.normal.dot(new_rel_vel - old_rel_vel);

    //                 other_contact.target_delta_velocity.into()
    //             });
    //         }
    //     }
    // }
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
