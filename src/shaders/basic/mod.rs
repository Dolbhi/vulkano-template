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
