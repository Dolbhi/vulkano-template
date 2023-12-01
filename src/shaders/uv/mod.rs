pub use super::basic::vs;

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/uv/fragment.frag",
    }
}
