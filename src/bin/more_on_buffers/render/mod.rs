mod render_data {
    pub mod material;
    pub mod mesh;
    pub mod render_object;
}
mod render_loop;
mod renderer;

use vulkano_template::shaders::basic;

pub use render_loop::RenderLoop;
pub type UniformData = basic::vs::Data;
