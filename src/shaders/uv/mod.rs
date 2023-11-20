pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/uv/vertex.vert",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/uv/fragment.frag",
    }
}
