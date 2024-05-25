pub mod draw;

vulkano_shaders::shader! {
    shaders: {
        // draw
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
        grad_fs: {
            ty: "fragment",
            path: "src/shaders/draw/gradient/fragment.frag",
        },

        // colored draw
        colored_vs: {
            ty: "vertex",
            path: "src/shaders/colored/vertex.vert",
        },
        solid_fs: {
            ty: "fragment",
            path: "src/shaders/colored/solid.frag",
        },
        billboard_vs: {
            ty: "vertex",
            path: "src/shaders/colored/billboard.vert",
        },

        // lighting
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
