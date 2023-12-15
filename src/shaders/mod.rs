pub mod draw {
    vulkano_shaders::shader! {
        shaders: {
            basic_vs: {
                ty: "vertex",
                path: "src/shaders/basic/vertex.vert",
            },
            basic_fs: {
                ty: "fragment",
                path: "src/shaders/basic/fragment.frag",
            },
            phong_vs: {
                ty: "vertex",
                path: "src/shaders/phong/vertex.vert",
            },
            phong_fs: {
                ty: "fragment",
                path: "src/shaders/phong/fragment.frag",
            },
            uv_fs: {
                ty: "fragment",
                path: "src/shaders/uv/fragment.frag",
            },
            alpha_fs: {
                ty: "fragment",
                path: "src/shaders/alpha/fragment.frag",
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
}
