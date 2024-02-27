mod camera;
// pub mod component_info;
pub mod light;
pub mod object_loader;
mod scene;
pub mod transform;

pub use camera::Camera;
// pub use

// use crate::render::RenderObject;

#[derive(Debug)]
pub struct NameComponent(pub String);
