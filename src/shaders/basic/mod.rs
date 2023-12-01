pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/basic/vertex.vert",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/basic/fragment.frag",
    }
}

use cgmath::{Matrix, Matrix4, Transform};
impl From<Matrix4<f32>> for vs::GPUObjectData {
    fn from(value: Matrix4<f32>) -> Self {
        vs::GPUObjectData {
            render_matrix: value.into(),
            normal_matrix: value.inverse_transform().unwrap().transpose().into(),
        }
    }
}
