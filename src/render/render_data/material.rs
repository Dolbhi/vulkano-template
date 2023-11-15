use std::sync::Arc;

use crate::vulkano_objects::pipeline;
use vulkano::{
    device::Device,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::RenderPass,
    shader::EntryPoint,
};

pub struct Material {
    vs: EntryPoint,
    fs: EntryPoint,
    pipeline: Arc<GraphicsPipeline>,
}

impl Material {
    pub fn new(vs: EntryPoint, fs: EntryPoint, pipeline: Arc<GraphicsPipeline>) -> Self {
        Material { vs, fs, pipeline }
    }

    pub fn get_pipeline(&self) -> &Arc<GraphicsPipeline> {
        &self.pipeline
    }

    pub fn recreate_pipeline(
        &mut self,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) {
        self.pipeline = pipeline::window_size_dependent_pipeline(
            device,
            self.vs.clone(),
            self.fs.clone(),
            viewport,
            render_pass,
        );
    }
}
