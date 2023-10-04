use cgmath::Matrix4;
use std::sync::Arc;

use vulkano::buffer::{BufferContents, Subbuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
    SubpassContents,
};
use vulkano::device::Queue;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::Framebuffer;

use super::allocators::Allocators;
use crate::vulkano_objects::buffers::Buffers;
use crate::Vertex2d;

pub fn create_only_vertex_command_buffers(
    allocators: &Allocators,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: &[Arc<Framebuffer>],
    vertex_buffer: Subbuffer<[Vertex2d]>,
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    framebuffers
        .iter()
        .map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                &allocators.command_buffer,
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            )
            .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into())],
                        ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            Arc::new(builder.build().unwrap())
        })
        .collect()
}

/// Creates a command buffer for each framebuffer with the given pipeline and corresponding vertex, index and uniform buffers
pub fn create_simple_command_buffers<U: BufferContents + Clone>(
    allocators: &Allocators,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: &[Arc<Framebuffer>],
    buffers: &Buffers<U>,
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    framebuffers
        .iter()
        .enumerate()
        .map(|(i, framebuffer)| {
            let mut builder = AutoCommandBufferBuilder::primary(
                &allocators.command_buffer,
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            )
            .unwrap();

            let index_buffer = buffers.get_index();
            let index_buffer_length = index_buffer.len();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into())],
                        ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    buffers.get_uniform_descriptor_set(i),
                )
                .bind_vertex_buffers(0, buffers.get_vertex())
                .bind_index_buffer(index_buffer)
                .draw_indexed(index_buffer_length as u32, 1, 0, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            Arc::new(builder.build().unwrap())
        })
        .collect()
}

/// Creates a command buffer for each framebuffer with the given pipeline and corresponding vertex, index and uniform buffers
pub fn create_simple_command_buffers_2<U: BufferContents + Clone>(
    allocators: &Allocators,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: &[Arc<Framebuffer>],
    buffers: &Buffers<U>,
    bg_colour: [f32; 4],
    radians: cgmath::Rad<f32>,
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    let cam_pos = cgmath::vec3(0., 0., -2.);
    let view = Matrix4::from_translation(cam_pos);
    let mut projection = cgmath::perspective(cgmath::Rad(1.2), 1., 0.1, 200.);
    projection.x.x *= -1.;
    let model = Matrix4::from_axis_angle(cgmath::vec3(0., 1., 0.), radians);

    let push_constants = MeshPushConstants {
        data: [0., 0., 0., 0.],
        render_matrix: (projection * view * model).into(),
    };

    framebuffers
        .iter()
        .enumerate()
        .map(|(i, framebuffer)| {
            let mut builder = AutoCommandBufferBuilder::primary(
                &allocators.command_buffer,
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            )
            .unwrap();

            let index_buffer = buffers.get_index();
            let index_buffer_length = index_buffer.len();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some(bg_colour.into())],
                        ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    buffers.get_uniform_descriptor_set(i),
                )
                .push_constants(pipeline.layout().clone(), 0, push_constants.clone())
                .bind_vertex_buffers(0, buffers.get_vertex())
                .bind_index_buffer(index_buffer)
                .draw_indexed(index_buffer_length as u32, 1, 0, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            Arc::new(builder.build().unwrap())
        })
        .collect()
}

// use vulkano::command_buffer::pool::{CommandPool, CommandPoolCreateInfo};
// use vulkano::device::Device;

// pub fn create_command_pool(device: Arc<Device>, queue: Arc<Queue>) -> Arc<CommandPool> {
//     let command_pool = CommandPool::new(
//         device,
//         CommandPoolCreateInfo {
//             queue_family_index: queue.queue_family_index(),
//             reset_command_buffer: true,
//             ..Default::default()
//         },
//     )
//     .unwrap();

//     // allocate buffers
//     command_pool
//         .allocate_command_buffers(Default::default())
//         .unwrap()
//         .map(|allocator| {});

//     Arc::new(command_pool)
// }

#[derive(BufferContents, Clone)]
#[repr(C)]
struct MeshPushConstants {
    data: [f32; 4],
    render_matrix: [[f32; 4]; 4],
}
