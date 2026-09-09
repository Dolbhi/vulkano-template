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
    pub resources: Option<GameResources>,
}

pub struct GameResources {
    pub transforms: TransformSystem,
    pub colliders: ColliderSystem,
    pub camera: Camera,
    pub last_delta_time: f32,
    pub fixed_seconds: f32,
    pub inputs: InputState,
}

/// Wrapper for last delta time and total game time, both in seconds
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
                last_delta_time: 0.,
                fixed_seconds: 0.,
                inputs: InputState::default(),
            }),
        }
    }

    pub fn update_time(&mut self, seconds_passed: f32) {
        if let Some(resources) = self.resources.as_mut() {
            resources.fixed_seconds += seconds_passed;
            resources.last_delta_time = seconds_passed;
        }
    }

    pub fn execute_schedule(&mut self, schedule: &mut Schedule) {
        let GameResources {
            transforms,
            colliders,
            camera,
            last_delta_time,
            fixed_seconds,
            inputs,
        } = self.resources.take().unwrap();

        let mut sch_resources = Resources::default();
        sch_resources.insert(transforms);
        sch_resources.insert(colliders);
        sch_resources.insert(camera);
        sch_resources.insert(GameTime(last_delta_time, fixed_seconds));
        sch_resources.insert(inputs);

        schedule.execute(&mut self.world, &mut sch_resources);

        let GameTime(last_delta_time, fixed_seconds) = sch_resources.remove().unwrap();
        let resources = GameResources {
            transforms: sch_resources.remove().unwrap(),
            colliders: sch_resources.remove().unwrap(),
            camera: sch_resources.remove().unwrap(),
            last_delta_time,
            fixed_seconds,
            inputs: sch_resources.remove().unwrap(),
        };

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
