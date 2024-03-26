mod camera;
mod game_world;
pub mod light;
pub mod transform;

pub use camera::Camera;
pub use game_world::{GameWorld, Inputs};

use cgmath::{Rad, Vector3};

#[derive(Debug)]
pub struct NameComponent(pub String);
pub struct Rotate(pub Vector3<f32>, pub Rad<f32>);
pub struct DisabledLERP;
