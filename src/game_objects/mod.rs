mod camera;
pub mod light;
mod scene;
pub mod transform;

pub use camera::Camera;
use cgmath::{Rad, Vector3};
// pub use

// use crate::render::RenderObject;

#[derive(Debug)]
pub struct NameComponent(pub String);

pub struct Rotate(pub Vector3<f32>, pub Rad<f32>);

pub struct FollowCamera(pub Vector3<f32>);
