use std::sync::{Arc, RwLock};

use cgmath::{Quaternion, Rotation3};

use crate::{
    input::InputState,
    physics::{ColliderSystem, LeafInHierachy, RigidBody},
    LOGIC_PROFILER,
};

use super::{
    transform::{TransformID, TransformSystem},
    Camera, Rotate, TransformTracker,
};
use legion::*;

pub const CAM_SPEED: f32 = 6.;
pub const SLOW_COEFF: f32 = 0.1;

/// stores game data and handles logic updates
pub struct GameWorld {
    pub world: World,
    pub resources : Option<GameResources>
}

pub struct GameResources {
    pub transforms: TransformSystem,
    pub colliders: ColliderSystem,
    pub camera: Camera,
    pub fixed_seconds: f32,
    pub last_delta_time: f32,
    pub inputs: InputState,
}

/// Wrapper for game time and last delta time, both in seconds
pub struct GameTime(pub f32, pub f32);

impl GameWorld {
    pub fn new() -> Self {
        let mut transforms = TransformSystem::new();
        let colliders = ColliderSystem::new();
        let mut world = World::default();
        let camera = Camera::from_transform(transforms.next().unwrap());
        world.push((camera.transform,));

        // colliders.

        Self {
            world,
            resources: Some(GameResources {
                transforms,
                colliders,
                camera,
                fixed_seconds: 0.,
                last_delta_time: 0.,
                inputs: InputState::default(),
            })
        }
    }

    /// update world logic with a time step
    ///
    /// # Order
    /// 1. Rigidbody movement
    /// 2. Collision resolution
    /// 3. Other logic
    pub fn update(&mut self, seconds_passed: f32) {
        let mut resources = self.resources.take().unwrap();
        resources.last_delta_time = seconds_passed;
        resources.fixed_seconds += seconds_passed;

        // let mut profiler = unsafe { LOGIC_PROFILER.lock().unwrap() };
        let logic_start = std::time::Instant::now();

        // physics update
        let mut query = <(&TransformID, &mut Arc<RwLock<RigidBody>>)>::query();
        query.for_each_mut(&mut self.world, |(transfrom, rigid_body)| {
            rigid_body.write().unwrap().update(
                resources.transforms.get_transform_mut(transfrom).unwrap(),
                seconds_passed,
            );
            // println!(
            //     "[RB] id: {:?}, model: {:?}",
            //     transfrom,
            //     self.transforms.get_global_model(transfrom)
            // );
        });

        // [Profiling] Physics
        let phys_time = logic_start.elapsed().as_micros() as u32;
        let coll_start = std::time::Instant::now();

        // update bounds
        <(&TransformID, &mut LeafInHierachy)>::query().for_each_mut(&mut self.world, |(id, collider)| {
            if let Some(transform) = resources.transforms.get_transform(id) {
                if transform.needs_coll_update {
                    resources.colliders.update(collider, &mut resources.transforms);
                    resources.transforms.reset_coll_update(id);
                }
            }
        });

        let contact_resolver = resources.colliders.get_contacts(&mut resources.transforms);
        contact_resolver.resolve(&mut resources.transforms, seconds_passed);
        // store old velocity
        <&mut Arc<RwLock<RigidBody>>>::query().for_each_mut(&mut self.world, |rb|rb.write().unwrap().set_old_velocity());

        // [Profiling] Colliders
        let coll_time = coll_start.elapsed().as_micros() as u32;
        let lerp_start = std::time::Instant::now();

        // update interpolation models
        <&TransformID>::query().for_each(&self.world, |transform_id| {
            // *last_model =
            //     InterpolateTransform(self.transforms.get_global_model(transform_id).unwrap());
            if resources.transforms.store_last_model(transform_id).is_err() {
                println!("[Error] Failed to find transform of interpolated object");
            }
        });
        resources.transforms.update_last_fixed();

        // [Profiling] Interpolation
        let lerp_time = lerp_start.elapsed().as_micros() as u32;
        let others_start = std::time::Instant::now();

        // move cam
        resources.inputs.move_transform(
            resources.transforms
                .get_transform_mut(&resources.camera.transform)
                .unwrap(),
            seconds_passed,
            CAM_SPEED,
            SLOW_COEFF,
        );

        // update rotate
        <(&TransformID, &Rotate)>::query().for_each_mut(&mut self.world, |(transform_id, rotate)|
        {
            let transform = resources.transforms.get_transform_mut(transform_id).unwrap();
            transform.set_rotation(
                Quaternion::from_axis_angle(rotate.0, rotate.1 * seconds_passed)
                * transform.get_local_transform().rotation,
            );
        }
    );

        <(&TransformID, &TransformTracker)>::query().for_each_mut(&mut self.world, |(transform_id, TransformTracker(tag))| {
            let model = resources.transforms.get_global_model(transform_id).unwrap();
            println!("[Transform] {}: {:?}", tag, model);
        });

        let mut profiler = LOGIC_PROFILER.lock().unwrap();
        profiler.add_sample(phys_time, 1);
        profiler.add_sample(coll_time, 2);
        profiler.add_sample(lerp_time, 3);
        profiler.add_sample(others_start.elapsed().as_micros() as u32, 4);

        self.resources = Some(resources);
    }

    /// clear the world and transforms and reset the camera
    pub fn clear(&mut self) {
        *self = Self::new();
        // self.world.clear();
        // self.transforms = TransformSystem::new();
        // self.camera = Camera::from_transform(self.transforms.next().unwrap());
        // self.world.push((self.camera.transform,));
    }
}
impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}
