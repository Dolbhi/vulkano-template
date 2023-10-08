mod material;
mod mesh;
mod render_loop;
mod render_object;
mod renderer;

use vulkano_template::shaders::basic;

pub use render_loop::RenderLoop;
pub type UniformData = basic::vs::Data;
