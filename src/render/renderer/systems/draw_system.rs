// mod frame_data;
// use frame_data::FrameData;

use std::sync::Arc;

use cgmath::Matrix4;
use vulkano::{
    buffer::{BufferContents, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{layout::DescriptorSetLayout, DescriptorSetsCollection},
    render_pass::Subpass,
    shader::EntryPoint,
};

use crate::{
    render::{context::Context, render_data::material::Shader},
    vulkano_objects::{buffers::write_to_storage_buffer, pipeline::PipelineHandler},
};

/// Collection of pipelines and associated rendering data
///
/// All pipelines share sets 0 and 1, describing scene data and an array of object data (storage buffer) respectively
///
/// Materials can optionally add more sets, starting from set 2
pub struct DrawSystem<const COUNT: usize> {
    pub shaders: [Shader; COUNT],
}

impl<'a, const COUNT: usize> DrawSystem<COUNT> {
    /// creates from a collection of shader entry points
    pub fn new(
        context: &Context,
        subpass: &Subpass,
        shaders: [(EntryPoint, EntryPoint); COUNT],
    ) -> (Self, [Arc<DescriptorSetLayout>; 2]) {
        let shaders: [Shader; COUNT] = shaders
            .map(|(vs, fs)| {
                PipelineHandler::new(
                    context.device.clone(),
                    vs,
                    fs,
                    context.viewport.clone(),
                    subpass.clone(),
                    [], // [(0, 0)],
                    crate::vulkano_objects::pipeline::PipelineType::Drawing,
                )
            })
            .map(Shader::new);

        let layouts = shaders[0].pipeline.layout().set_layouts();
        let layouts = [layouts[0].clone(), layouts[1].clone()];

        (DrawSystem { shaders }, layouts)
    }

    // requires #![feature(generic_const_exprs)]
    // pub fn extend<const EXTEND: usize>(
    //     self,
    //     context: &Context,
    //     subpass: &Subpass,
    //     shaders: [(EntryPoint, EntryPoint); EXTEND],
    // ) -> DrawSystem<{ COUNT + EXTEND }> {
    //     let new_pipelines = shaders.map(|(vs, fs)| {
    //         PipelineGroup::new(PipelineHandler::new(
    //             context.device.clone(),
    //             vs,
    //             fs,
    //             context.viewport.clone(),
    //             subpass.clone(),
    //             [], // [(0, 0)],
    //             crate::vulkano_objects::pipeline::PipelineType::Drawing,
    //         ))
    //     });

    //     let pipelines = [0; COUNT + EXTEND].map(|n| {
    //         if n < COUNT {
    //             self.pipelines[n]
    //         } else {
    //             new_pipelines[n - COUNT]
    //         }
    //     });
    //     DrawSystem { pipelines }
    // }

    /// Recreate all pipelines with any changes in viewport
    ///
    /// See also: [recreate_pipeline](PipelineHandler::recreate_pipeline)
    pub fn recreate_pipelines(&mut self, context: &Context) {
        for pipeline in self.shaders.iter_mut() {
            pipeline
                .pipeline
                .recreate_pipeline(context.device.clone(), context.viewport.clone());
        }
    }

    /// sort and write object data to given storage buffer (must be called before rendering)
    pub fn update_object_buffer<O: BufferContents + From<Matrix4<f32>>>(
        &mut self,
        buffer: &Subbuffer<[O]>,
    ) {
        let obj_iter = self
            .shaders
            .iter_mut()
            .flat_map(|pipeline| pipeline.upload_pending_objects());
        write_to_storage_buffer(buffer, obj_iter);
    }
    /// bind draw calls to the given command buffer builder, be sure to call `update_object_buffer()` before hand
    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        // image_i: usize,
        sets: impl DescriptorSetsCollection + Clone,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        let mut object_index = 0;
        for pipeline_group in self.shaders.iter_mut() {
            pipeline_group.draw_objects(
                &mut object_index,
                sets.clone(), //self.frame_data[image_i].descriptor_sets.clone(),
                command_builder,
                // &mut self.pending_objects,
            );
        }
    }

    // fn clear_unused_resource(&mut self) {
    //     for pipeline_group in self.pipelines.iter() {
    //         for material in pipeline_group.materials.iter() {
    //             if self.pending_objects
    //         }
    //     }
    // }
}
