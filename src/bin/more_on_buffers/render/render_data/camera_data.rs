use cgmath::Matrix4;

pub struct CameraData {
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
    view_proj: Matrix4<f32>,
}

impl CameraData {}
