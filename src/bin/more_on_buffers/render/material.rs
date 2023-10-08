use std::sync::Arc;

use vulkano::{
    device::Device,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::RenderPass,
    shader::ShaderModule,
};
use vulkano_template::vulkano_objects::pipeline;

struct Material {
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
    pipeline: Arc<GraphicsPipeline>,
}

impl Material {
    pub fn clone_pipeline(&self) -> Arc<GraphicsPipeline> {
        self.pipeline.clone()
    }

    pub fn recreate_pipeline(
        &mut self,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) {
        self.pipeline = pipeline::create_pipeline(
            device,
            self.vs.clone(),
            self.fs.clone(),
            render_pass,
            viewport,
        );
    }
}
