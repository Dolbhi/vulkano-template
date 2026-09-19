use std::sync::{Arc, RwLock};

use cgmath::{InnerSpace, Vector2, Vector3};

use crate::{game_objects::{RunnableMut, transform::TransformID}, physics::RigidBody};

const WALKER_MAX_GROUND_SEP: f32 = 0.2;

/// Attempt to move at target_vel relative to the ground
pub struct Walker {
    pub target_vel: Vector2<f32>,
    pub max_friction: f32,
    pub feet_pos: Vector3<f32>,
}

impl RunnableMut for (&TransformID, &Walker, &mut Arc<RwLock<RigidBody>>) {
    fn update(self, resources: &mut super::GameResources) {
        let model = resources.transforms.get_global_model(self.0).expect("Cannot find transform associated with walker");
        if let Some((point, coll)) = resources.colliders.raycast(
            &mut resources.transforms,
            (model * self.1.feet_pos.extend(1.)).truncate(),
            -model.y.truncate(), // only checks down
            WALKER_MAX_GROUND_SEP,
        ) {
            let ground_vel = if let Some(ground_rb) = coll.get_rigidbody() {
                ground_rb.read().unwrap().velocity
            } else {
                [0., 0., 0.].into()
            };

            let mut rb_guard = self.2.write().unwrap();
            // cannot move inf mass
            if rb_guard.inv_mass < f32::EPSILON {
                return;
            }

            let hori_rel_vel = {
                let dv = rb_guard.velocity - ground_vel;
                Vector2::new(dv.x, dv.z)
            };

            let target_impulse = (self.1.target_vel - hori_rel_vel) / rb_guard.inv_mass;
            let target_imp_mag = target_impulse.magnitude();
            let target_imp_dir = target_impulse / target_imp_mag;
            let clamped_impulse = self.1.max_friction.min(target_imp_mag) * target_imp_dir;

            rb_guard.apply_impulse_rel([0., 0., 0.].into(), [clamped_impulse.x, 0., clamped_impulse.y].into(), [model.x, model.y, model.z]);
        };
    }
}