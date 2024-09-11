mod deferred_renderer;
pub mod systems {
    mod bounding_box_system;
    mod draw_system;
    mod lighting_system;

    pub use bounding_box_system::BoundingBoxSystem;
    pub use draw_system::DrawSystem;
    pub use lighting_system::LightingSystem;
}

pub use deferred_renderer::DeferredRenderer;

use crate::render::Context;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};

pub trait Renderer {
    /// Adds draw calls to the given command buffer builder
    fn build_command_buffer(
        &mut self,
        index: usize,
        command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    );
    /// Triggers pipeline recreation as needed (such as during screen resize)
    fn recreate_pipelines(&mut self, context: &Context);
    /// Triggers framebuffer recreation as needed (such as during screen resize)
    fn recreate_framebuffers(&mut self, context: &Context);
}
