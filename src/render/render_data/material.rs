// use std::sync::Arc;

// use vulkano::{
//     command_buffer::AutoCommandBufferBuilder, descriptor_set::PersistentDescriptorSet,
//     pipeline::PipelineLayout,
// };

// pub struct Material {
//     pub pipeline_id: String,
//     pub material_sets: Vec<Arc<PersistentDescriptorSet>>,
//     pub layout: Arc<PipelineLayout>,
// }

// impl Material {
//     pub fn bind_sets<T, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
//         &self,
//         command_builder: &mut AutoCommandBufferBuilder<T, A>,
//     ) {
//         for (i, set) in self.material_sets.iter().enumerate() {
//             command_builder
//                 .bind_descriptor_sets(
//                     vulkano::pipeline::PipelineBindPoint::Graphics,
//                     self.layout.clone(),
//                     i as u32 + 2,
//                     set.clone(),
//                 )
//                 .unwrap();
//         }
//     }
// }
