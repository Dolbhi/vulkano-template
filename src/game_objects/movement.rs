use std::sync::{Arc, RwLock};

use cgmath::{InnerSpace, Vector2, Vector3, Zero};

use crate::{game_objects::{RunnableMut, transform::TransformID}, physics::RigidBody};

const WALKER_MAX_GROUND_SEP: f32 = 0.2;
const PLAYER_WALK_VEL: f32 = 6.0;

/// Attempt to move at target_vel relative to the ground
pub struct Walker {
    pub target_vel: Vector2<f32>,
    pub max_friction: f32,
    pub rel_feet_pos: Vector3<f32>,
}

/// Walker entity will be player controlled
pub struct PlayerWalkerController;

impl RunnableMut for (&TransformID, &Walker, &mut Arc<RwLock<RigidBody>>) {
    fn update(self, resources: &mut super::GameResources) {
        let model = resources.transforms.get_global_model(self.0).expect("Cannot find transform associated with walker");
        let rb_rotation = resources.transforms.get_global_rotation(self.0).unwrap();
        if let Some((_, coll)) = resources.colliders.raycast(
            &mut resources.transforms,
            (model * self.1.rel_feet_pos.extend(1.)).truncate(),
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

            if target_imp_mag <= f32::EPSILON {
                return;
            }

            let clamped_impulse = target_impulse.normalize_to(self.1.max_friction.min(target_imp_mag));
            rb_guard.apply_impulse_rel([0., 0., 0.].into(), [clamped_impulse.x, 0., clamped_impulse.y].into(), rb_rotation);
        };
    }
}

impl RunnableMut for (&TransformID, &mut Walker, &PlayerWalkerController) {
    fn update(self, resources: &mut super::GameResources) {
        let mut movement = Vector3::zero();
        // let mut y_movement = 0.;
        if resources.inputs.w.get_was_pressed() {
            movement.z -= 1.; // forward
        } else if resources.inputs.s.get_was_pressed() {
            movement.z += 1.; // backwards
        }
        if resources.inputs.a.get_was_pressed() {
            movement.x -= 1.; // left
        } else if resources.inputs.d.get_was_pressed() {
            movement.x += 1.; // right
        }

        if movement.is_zero() {
            self.1.target_vel = [0., 0.].into();
            return;
        }
        
        let movement = resources.transforms.get_global_rotation(self.0).unwrap() * movement;
        let movement = Vector2::new(movement.x, movement.z);
        self.1.target_vel = movement.normalize_to(PLAYER_WALK_VEL);
    }
}