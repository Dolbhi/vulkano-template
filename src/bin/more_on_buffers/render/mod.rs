mod render_loop;
mod render_object;
mod renderer;

use vulkano_template::shaders::movable_square;

pub use render_loop::RenderLoop;
pub type UniformData = movable_square::vs::Data;
