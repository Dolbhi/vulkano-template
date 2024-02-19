// const SHADER_ROOT: &str = "src/shaders/draw/";

vulkano_shaders::shader! {
    shaders: {
        basic_vs: {
            ty: "vertex",
            path: "src/shaders/draw/basic/vertex.vert",
        },
        basic_fs: {
            ty: "fragment",
            path: "src/shaders/draw/basic/fragment.frag",
        },
        uv_fs: {
            ty: "fragment",
            path: "src/shaders/draw/uv/fragment.frag",
        },
        solid_fs: {
            ty: "fragment",
            path: "src/shaders/draw/solid/fragment.frag",
        },
    }
}

use cgmath::{Matrix, Matrix4, Transform};
use winit::dpi::PhysicalSize;
impl From<Matrix4<f32>> for GPUObjectData {
    fn from(value: Matrix4<f32>) -> Self {
        GPUObjectData {
            render_matrix: value.into(),
            normal_matrix: value.inverse_transform().unwrap().transpose().into(),
        }
    }
}

use crate::game_objects::Camera;
impl GPUGlobalData {
    pub fn from_camera(camera: &Camera, extends: PhysicalSize<u32>) -> Self {
        let aspect = extends.width as f32 / extends.height as f32;
        let proj = camera.projection_matrix(aspect);
        let view = camera.view_matrix();
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
