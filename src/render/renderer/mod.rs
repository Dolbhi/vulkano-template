mod deferred_renderer;

pub use deferred_renderer::DeferredRenderer;

use crate::render::Context;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};

pub trait Renderer {
    fn build_command_buffer(
        &mut self,
        index: usize,
        command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    );
    fn recreate_pipelines(&mut self, context: &Context);
    fn recreate_framebuffers(&mut self, context: &Context);
}
