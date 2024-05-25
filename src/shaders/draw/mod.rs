use super::{GPUGlobalData, GPUObjectData};
use crate::game_objects::Camera;
use cgmath::{Matrix, Matrix4, Transform};
use winit::dpi::PhysicalSize;

impl From<Matrix4<f32>> for GPUObjectData {
    fn from(value: Matrix4<f32>) -> Self {
        GPUObjectData {
            render_matrix: value.into(),
            normal_matrix: value.inverse_transform().unwrap().transpose().into(),
            // color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl GPUGlobalData {
    pub fn from_camera(camera: &Camera, extends: PhysicalSize<u32>) -> Self {
        let aspect = extends.width as f32 / extends.height as f32;
        let proj = camera.projection_matrix(aspect);
        let view = camera.view_matrix(); //model.inverse_transform().unwrap();
        let view_proj = proj * view;
        let inv_view_proj = view_proj.inverse_transform().unwrap();

        GPUGlobalData {
            view: view.into(),
            proj: proj.into(),
            view_proj: view_proj.into(),
            inv_view_proj: inv_view_proj.into(),
        }
    }
}
// default for buffer init, invalid otherwise
impl Default for GPUGlobalData {
    fn default() -> Self {
        Self {
            view: Default::default(),
            proj: Default::default(),
            view_proj: Default::default(),
            inv_view_proj: Default::default(),
        }
    }
}
