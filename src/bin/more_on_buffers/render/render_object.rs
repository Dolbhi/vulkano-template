use cgmath::{Matrix4, SquareMatrix};
use std::sync::Arc;

use vulkano::{
    buffer::BufferContents, descriptor_set::layout::DescriptorSetLayout, device::Queue,
    pipeline::GraphicsPipeline,
};
use vulkano_template::{
    models::Mesh,
    vulkano_objects::{
        allocators::Allocators,
        buffers::{create_cpu_accessible_uniforms, Buffers, Uniform},
    },
};

pub struct RenderObject<U: BufferContents + Clone> {
    buffers: Buffers,
    uniforms: Vec<Uniform<U>>,
    pipeline: Arc<GraphicsPipeline>,
    transform_matrix: Matrix4<f32>,
}

impl<U: BufferContents + Clone> RenderObject<U> {
    pub fn new(
        allocators: &Allocators,
        transfer_queue: Arc<Queue>,
        mesh: Mesh,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        uniform_buffer_count: usize,
        initial_uniform: U,
        pipeline: Arc<GraphicsPipeline>,
    ) -> Self {
        let buffers = Buffers::initialize_device_local(allocators, transfer_queue, mesh);
        let uniforms = create_cpu_accessible_uniforms(
            allocators,
            descriptor_set_layout,
            uniform_buffer_count,
            initial_uniform,
        );
        // let material = create_pipeline(device, vs, fs, render_pass, viewport)

        Self {
            buffers,
            uniforms,
            pipeline,
            transform_matrix: Matrix4::identity(),
        }
    }

    pub fn get_buffers(&self) -> &Buffers {
        &self.buffers
    }

    pub fn get_uniforms(&self) -> &Vec<Uniform<U>> {
        &self.uniforms
    }

    pub fn get_pipeline(&self) -> Arc<GraphicsPipeline> {
        return self.pipeline.clone();
    }

    pub fn replace_pipeline(&mut self, new_pipeline: Arc<GraphicsPipeline>) {
        self.pipeline = new_pipeline;
    }
}
