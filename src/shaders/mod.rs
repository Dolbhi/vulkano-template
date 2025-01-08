pub mod draw;

// #[derive(Default)]
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

        // old colored draw
        colored_vs: {
            ty: "vertex",
            path: "src/shaders/draw/colored/vertex.vert",
        },
        solid_fs: {
            ty: "fragment",
            path: "src/shaders/draw/colored/solid.frag",
        },
        billboard_vs: {
            ty: "vertex",
            path: "src/shaders/draw/colored/billboard.vert",
        },

        // new colored draw
        new_colored_vs: {
            ty: "vertex",
            path: "src/shaders/colored/vertex.vert",
        },
        new_solid_fs: {
            ty: "fragment",
            path: "src/shaders/colored/solid.frag",
        },
        new_billboard_vs: {
            ty: "vertex",
            path: "src/shaders/colored/billboard.vert",
        },

        // axis aligned bounding box
        bounding_box_vs: {
            ty: "vertex",
            path: "src/shaders/bounding_box/box.vert"
        },
        bounding_box_fs: {
            ty: "fragment",
            path: "src/shaders/bounding_box/box.frag"
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
