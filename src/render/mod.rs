mod render_data {
    pub mod frame_data;
    pub mod material;
    pub mod mesh;
    pub mod render_object;
}
mod render_loop;
mod renderer;

pub use render_loop::RenderLoop;