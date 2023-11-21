use std::sync::Arc;

use crate::vulkano_objects::pipeline;
use vulkano::{
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::PersistentDescriptorSet,
    device::Device,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline, Pipeline},
    render_pass::RenderPass,
    shader::EntryPoint,
};

pub struct Material {
    vs: EntryPoint,
    fs: EntryPoint,
    pub pipeline: Arc<GraphicsPipeline>,
    pub material_sets: Vec<Arc<PersistentDescriptorSet>>,
}

impl Material {
    pub fn new(
        vs: EntryPoint,
        fs: EntryPoint,
        pipeline: Arc<GraphicsPipeline>,
        material_sets: Vec<Arc<PersistentDescriptorSet>>,
    ) -> Self {
        Material {
            vs,
            fs,
            pipeline,
            material_sets,
        }
    }

    pub fn bind_sets<T, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &self,
        command_builder: &mut AutoCommandBufferBuilder<T, A>,
    ) {
        for (i, set) in self.material_sets.iter().enumerate() {
            command_builder
                .bind_descriptor_sets(
                    vulkano::pipeline::PipelineBindPoint::Graphics,
                    self.pipeline.layout().clone(),
                    i as u32 + 2,
                    set.clone(),
                )
                .unwrap();
        }
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
