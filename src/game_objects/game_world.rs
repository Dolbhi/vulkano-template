use std::sync::{Arc, RwLock};

use crate::{
    LOGIC_PROFILER, game_objects::movement::{PlayerWalkerController, Walker}, input::InputState, physics::{ColliderSystem, LeafInHierachy, RigidBody},
};

use super::{
    transform::{TransformID, TransformSystem},
    Camera, Rotate, TransformTracker,
};
use legion::*;

// pub const CAM_SPEED: f32 = 6.;
// pub const SLOW_COEFF: f32 = 0.1;

macro_rules! run_update_mut {
    ($game_world:expr, $type:ty) => {
        <$type>::query().for_each_mut(&mut $game_world.world, |comp| <$type>::update(comp, &mut $game_world.resources));
    };
}
macro_rules! run_update {
    ($game_world:expr, $type:ty) => {
        <$type>::query().for_each(&$game_world.world, |comp| <$type>::update(comp, &mut $game_world.resources));
    };
}

/// stores game data and handles logic updates
pub struct GameWorld {
    pub world: World,
    pub resources: GameResources
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

pub trait RunnableMut : IntoQuery {
    fn update(self, resources: &mut GameResources);

    // // the thing we need macros for unfortunately
    // fn test(gameworld: &mut GameWorld, resources: &mut GameResources) {
    //     Self::query().for_each_mut(&mut gameworld.world, |c| Self::update(c, resources));
    // }
}
pub trait Runnable : IntoQuery {
    fn update(self, resources: &GameResources);

    // // the thing we need macros for unfortunately
    // fn test(gameworld: &mut GameWorld, resources: &GameResources) {
    //     Self::query().for_each(&mut gameworld.world, |c| Self::update(c, resources));
    // }
}

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
            resources: GameResources {
                transforms,
                colliders,
                camera,
                fixed_seconds: 0.,
                last_delta_time: 0.,
                inputs: InputState::default(),
            }
        }
    }

    /// update world logic with a time step
    ///
    /// # Order
    /// 1. Rigidbody movement
    /// 2. Collision resolution
    /// 3. Other logic
    pub fn update(&mut self, seconds_passed: f32) {
        self.resources.last_delta_time = seconds_passed;
        self.resources.fixed_seconds += seconds_passed;

        // let mut profiler = unsafe { LOGIC_PROFILER.lock().unwrap() };
        let logic_start = std::time::Instant::now();

        // physics update
        run_update!(self, (&TransformID, &Rotate));
        run_update_mut!(self, (&TransformID, &mut Walker, &PlayerWalkerController));
        run_update_mut!(self, (&TransformID, &Walker, &mut Arc<RwLock<RigidBody>>));
        run_update_mut!(self, (&TransformID, &mut Arc<RwLock<RigidBody>>));

        // [Profiling] Physics
        let phys_time = logic_start.elapsed().as_micros() as u32;
        let coll_start = std::time::Instant::now();

        // update bounds
        run_update_mut!(self, (&TransformID, &mut LeafInHierachy));

        let contact_resolver = self.resources.colliders.get_contacts(&mut self.resources.transforms);
        contact_resolver.resolve(&mut self.resources.transforms, seconds_passed);
        // store old velocity
        run_update_mut!(self, &mut Arc<RwLock<RigidBody>>);
        
        // [Profiling] Colliders
        let coll_time = coll_start.elapsed().as_micros() as u32;
        let lerp_start = std::time::Instant::now();

        // update interpolation models
        run_update!(self, &TransformID);
        self.resources.transforms.update_last_fixed();

        // [Profiling] Interpolation
        let lerp_time = lerp_start.elapsed().as_micros() as u32;
        let others_start = std::time::Instant::now();

        run_update!(self, (&TransformID, &TransformTracker));

        let mut profiler = LOGIC_PROFILER.lock().unwrap();
        profiler.add_sample(phys_time, 1);
        profiler.add_sample(coll_time, 2);
        profiler.add_sample(lerp_time, 3);
        profiler.add_sample(others_start.elapsed().as_micros() as u32, 4);
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