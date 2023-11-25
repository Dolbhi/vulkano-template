pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/phong/vertex.vert",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/phong/fragment.frag",
    }
}
