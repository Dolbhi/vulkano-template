mod camera;
mod game_world;
pub mod light;
pub mod transform;
pub mod utility;

pub use camera::Camera;
pub use game_world::{GameWorld, GameTime, GameResources};

use cgmath::{Quaternion, Rad, Vector3, Rotation3};
use legion::{World, system};

use crate::{game_objects::transform::TransformSystem, render::{
    RenderObject, RenderSubmit, resource_manager::{MaterialID, MeshID, ResourceRetriever},
}};

use self::transform::{TransformCreateInfo, TransformID};

#[derive(Debug)]
pub struct NameComponent(pub String);
pub struct Rotate(pub Vector3<f32>, pub Rad<f32>);

// /// flag indicating if object was moved/modified this frame
// pub struct PhysicsAwake(pub bool);

#[derive(Clone)]
pub struct MaterialSwapper<T: Clone> {
    materials: Vec<RenderSubmit<T>>,
    curent_index: usize,
}

pub struct TransformTracker<'a>(pub &'a str);

pub struct WorldLoader<'a, 'b: 'a> {
    pub world: &'a mut World,
    pub game_resources: &'a mut GameResources,
    pub resources: &'a mut ResourceRetriever<'b>,
}

#[system(for_each)]
pub fn update_rotate(transform_id: &TransformID, rotate: &Rotate, #[resource] transforms: &mut TransformSystem, #[resource] time: &GameTime)
{
    // update rotate
    let transform = transforms.get_transform_mut(transform_id).unwrap();
    transform.set_rotation(
        Quaternion::from_axis_angle(rotate.0, rotate.1 * time.0)
        * transform.get_local_transform().rotation,
    );
}

#[system(for_each)]
pub fn update_tracker(transform_id: &TransformID, TransformTracker(tag): &TransformTracker, #[resource] transforms: &mut TransformSystem) {
    let model = transforms.get_global_model(transform_id).unwrap();
    println!("[Transform] {}: {:?}", tag, model);
}

#[system(for_each)]
pub fn swap_material(swapper: &mut MaterialSwapper<()>, render_object: &mut RenderObject<()>) {
    // update basic mat swap
    let next_mat = swapper.swap_material();
    // println!("Swapped mat: {:?}", next_mat);
    render_object.material = next_mat;
}


impl<T: Clone> MaterialSwapper<T> {
    pub fn new(materials: impl IntoIterator<Item = RenderSubmit<T>>) -> Self {
        let materials = materials.into_iter().collect();
        Self {
            materials,
            curent_index: 0,
        }
    }

    pub fn swap_material(&mut self) -> RenderSubmit<T> {
        self.curent_index = (self.curent_index + 1) % self.materials.len();
        self.materials[self.curent_index].clone()
    }
}

impl<'a, 'b: 'a> WorldLoader<'a, 'b> {
    /// create a game object with just a transform and a render object components
    pub fn quick_ro(
        &mut self,
        transform: impl Into<TransformCreateInfo>,
        mesh: MeshID,
        material: MaterialID,
        lit: bool,
    ) -> (TransformID, legion::Entity) {
        let ro = self.resources.load_ro(mesh, material, lit);
        crate::load_transform_and_object!(self, transform, ro)
        // self.add_1_comp(transform, ro)
    }
}

/// create a new transform and load a new object with it
#[macro_export]
macro_rules! load_transform_and_object {
    ($loader:expr, $transform:expr, $($comp:expr),+) => {
        {
            let id = $loader.game_resources.transforms.add_transform($transform);
            (id, $loader.world.push((id, $($comp),+)))
        }
    };
}

/// load a new object with an arbitrary number of components
#[macro_export]
macro_rules! load_object {
    ($world:expr, $($comp:expr),+) => {
        $world.push(($($comp),+))
    };
}
