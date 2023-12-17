mod camera;
pub mod light;
mod scene;
pub mod transform;

pub use camera::Camera;
// pub use

// use crate::render::RenderObject;

#[derive(Debug)]
pub struct NameComponent(pub String);
