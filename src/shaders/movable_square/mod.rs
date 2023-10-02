pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/movable_square/vertex.vert",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/movable_square/fragment.frag",
    }
}
