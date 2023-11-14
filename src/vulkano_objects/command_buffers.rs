use std::sync::Arc;

// use rand::distributions::Uniform;
use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
};
// use vulkano::descriptor_set::{self, PersistentDescriptorSet};
use vulkano::device::Queue;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::Framebuffer;

use super::allocators::Allocators;
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
                    Default::default(),
                    // SubpassContents::Inline,
                )
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap()
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass(Default::default())
                .unwrap();

            builder.build().unwrap()
        })
        .collect()
}

// Creates a command buffer for each framebuffer with the given pipeline and corresponding vertex, index and uniform buffers
// pub fn create_simple_command_buffers<U: BufferContents + Clone>(
//     allocators: &Allocators,
//     queue: Arc<Queue>,
//     pipeline: Arc<GraphicsPipeline>,
//     framebuffers: &[Arc<Framebuffer>],
//     buffers: &Buffers<VertexFull>,
//     uniforms: &Vec<Uniform<U>>,
//     // render_object: &RenderObject<U>,
// ) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
//     framebuffers
//         .iter()
//         .enumerate()
//         .map(|(i, framebuffer)| {
//             let mut builder = AutoCommandBufferBuilder::primary(
//                 &allocators.command_buffer,
//                 queue.queue_family_index(),
//                 CommandBufferUsage::MultipleSubmit,
//             )
//             .unwrap();
//
//             let index_buffer = buffers.get_index();
//             let index_buffer_length = index_buffer.len();
//
//             builder
//                 .begin_render_pass(
//                     RenderPassBeginInfo {
//                         clear_values: vec![Some([0.1, 0.1, 0.1, 1.0].into()), Some(1.0.into())],
//                         ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
//                     },
//                     Default::default(),
//                     // SubpassContents::Inline,
//                 )
//                 .unwrap()
//                 .bind_pipeline_graphics(pipeline.clone())
//                 .unwrap()
//                 .bind_descriptor_sets(
//                     PipelineBindPoint::Graphics,
//                     pipeline.layout().clone(),
//                     0,
//                     uniforms[i].1.clone(),
//                 )
//                 .unwrap()
//                 .bind_vertex_buffers(0, buffers.get_vertex())
//                 .unwrap()
//                 .bind_index_buffer(index_buffer)
//                 .unwrap()
//                 .draw_indexed(index_buffer_length as u32, 1, 0, 0, 0)
//                 .unwrap()
//                 .end_render_pass(Default::default())
//                 .unwrap();
//
//             builder.build().unwrap()
//         })
//         .collect()
// }
//
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
