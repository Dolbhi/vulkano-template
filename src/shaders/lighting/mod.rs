pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/lighting/lighting.vert",
    }
}
pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/lighting/lighting.frag",
    }
}

vulkano_shaders::shader! {
    shaders: {
        point_vs: {
            ty: "vertex",
            path: "src/shaders/lighting/point.vert",
        },
        point_fs: {
            ty: "fragment",
            path: "src/shaders/lighting/point.frag",
        },
        direction_vs: {
            ty: "vertex",
            path: "src/shaders/lighting/directional.vert",
        },
        direction_fs: {
            ty: "fragment",
            path: "src/shaders/lighting/directional.frag",
        },
        ambient_fs: {
            ty: "fragment",
            path: "src/shaders/lighting/ambient.frag",
        },
    }
}
