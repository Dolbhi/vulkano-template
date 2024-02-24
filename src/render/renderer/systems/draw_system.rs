// mod frame_data;
// use frame_data::FrameData;

use std::sync::Arc;

use cgmath::Matrix4;
use vulkano::{
    buffer::{BufferContents, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{layout::DescriptorSetLayout, DescriptorSetsCollection},
    render_pass::Subpass,
    shader::{EntryPoint, ShaderModule},
};

use crate::{
    render::{context::Context, render_data::material::Shader},
    vulkano_objects::{buffers::write_to_storage_buffer, pipeline::PipelineHandler},
};

/// Collection of shaders, meant to be run on a single subpass
///
/// All shader pipelines share sets 0 and 1, describing global scene data and an array of object data (storage buffer) respectively
///
/// Materials can optionally add more sets, starting from set 2
pub struct DrawSystem {
    pub shaders: Vec<Shader>,
    subpass: Subpass,
}

impl<'a> DrawSystem {
    /// creates from a collection of shader entry points
    pub fn new(
        context: &Context,
        subpass: &Subpass,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
    ) -> (Self, [Arc<DescriptorSetLayout>; 2]) {
        let shader = Shader::new(PipelineHandler::new(
            context.device.clone(),
            vs.entry_point("main").unwrap(),
            fs.entry_point("main").unwrap(),
            context.viewport.clone(),
            subpass.clone(),
            [], // [(0, 0)],
            crate::vulkano_objects::pipeline::PipelineType::Drawing,
        ));

        let layouts = shader.pipeline.layout().set_layouts();
        let layouts = [layouts[0].clone(), layouts[1].clone()];

        (
            DrawSystem {
                shaders: vec![shader],
                subpass: subpass.clone(),
            },
            layouts,
        )
    }

    /// creates shader with the same subpass and dynamic bindings as this system, must be manually added later
    pub fn create_shader(
        &mut self,
        context: &Context,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
    ) -> Shader {
        Shader::new(PipelineHandler::new(
            context.device.clone(),
            vs.entry_point("main").unwrap(),
            fs.entry_point("main").unwrap(),
            context.viewport.clone(),
            self.subpass.clone(),
            [], // [(0, 0)],
            crate::vulkano_objects::pipeline::PipelineType::Drawing,
        ))
    }

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
