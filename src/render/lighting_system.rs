use vulkano::command_buffer::AutoCommandBufferBuilder;

use crate::{shaders::lighting, vulkano_objects::pipeline::PipelineHandler};

use super::Renderer;

pub struct LightingSystem {
    pipeline: PipelineHandler,
}

impl LightingSystem {
    pub fn new(context: &Renderer) -> Self {
        let vs = lighting::vs::load(context.device.clone())
            .expect("failed to create lighting shader module")
            .entry_point("main")
            .unwrap();
        let fs = lighting::fs::load(context.device.clone())
            .expect("failed to create lighting shader module")
            .entry_point("main")
            .unwrap();

        let pipeline = PipelineHandler::new(
            context.device.clone(),
            vs,
            fs,
            context.viewport.clone(),
            context.render_pass.clone(),
        );
        LightingSystem { pipeline }
    }

    // pub fn upload_lights(point lights, direction lights, ambient light)

    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        image_i: usize,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
    }
}
