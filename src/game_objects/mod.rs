mod camera;

pub use camera::Camera;

pub struct Transform(cgmath::Matrix4<f32>);
