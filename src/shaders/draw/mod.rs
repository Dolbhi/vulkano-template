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
        alpha_fs: {
            ty: "fragment",
            path: "src/shaders/draw/alpha/fragment.frag",
        }
    }
}

use cgmath::{Matrix, Matrix4, Transform};
impl From<Matrix4<f32>> for GPUObjectData {
    fn from(value: Matrix4<f32>) -> Self {
        GPUObjectData {
            render_matrix: value.into(),
            normal_matrix: value.inverse_transform().unwrap().transpose().into(),
        }
    }
}
