// mod frame_data;
// use frame_data::FrameData;

use std::sync::Arc;

use vulkano::{
    command_buffer::AutoCommandBufferBuilder, descriptor_set::DescriptorSetsCollection,
    render_pass::Subpass, shader::ShaderModule,
};

use crate::{
    render::{context::Context, render_data::material::Shader, resource_manager::MaterialID},
    vulkano_objects::pipeline::PipelineHandler,
};

/// Collection of shaders, meant to be run on a single subpass
///
/// All shader pipelines share sets 0 and 1, describing global scene data and an array of object data (storage buffer) respectively
///
/// Materials can optionally add more sets, starting from set 2
pub struct DrawSystem<T: Clone> {
    pub shaders: Vec<Shader<T>>,
    subpass: Subpass,
}

impl<T: Clone> DrawSystem<T> {
    /// creates from a collection of shader entry points
    pub fn new(
        context: &Context,
        subpass: &Subpass,
        id: MaterialID,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
    ) -> Self {
        let shader = Shader::new(
            id,
            PipelineHandler::new(
                context.device.clone(),
                vs.entry_point("main").unwrap(),
                fs.entry_point("main").unwrap(),
                context.viewport.clone(),
                subpass.clone(),
                [], // [(0, 0)],
                crate::vulkano_objects::pipeline::PipelineType::Drawing,
            ),
        );

        // let layouts = shader.pipeline.layout().set_layouts();
        // let layouts = [layouts[0].clone(), layouts[1].clone()];

        DrawSystem {
            shaders: vec![shader],
            subpass: subpass.clone(),
        }
    }

    /// creates shader with the same subpass and dynamic bindings as this system
    pub fn add_shader(
        &mut self,
        context: &Context,
        id: MaterialID,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
    ) {
        self.shaders.push(Shader::new(
            id,
            PipelineHandler::new(
                context.device.clone(),
                vs.entry_point("main").unwrap(),
                fs.entry_point("main").unwrap(),
                context.viewport.clone(),
                self.subpass.clone(),
                [], // [(0, 0)],
                crate::vulkano_objects::pipeline::PipelineType::Drawing,
            ),
        ));
    }

    /// search for shader via MaterialID
    pub fn find_shader(&mut self, id: MaterialID) -> Option<&mut Shader<T>> {
        self.shaders
            .iter_mut()
            .find(|shader| std::mem::discriminant(&shader.get_id()) == std::mem::discriminant(&id))
        // for shader in &mut self.shaders {
        //     if std::mem::discriminant(&shader.get_id()) == std::mem::discriminant(&id) {
        //         return Some(shader);
        //     }
        // }
        // None
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

    // /// sort and write object data to given storage buffer (must be called before rendering)
    // pub fn update_object_buffer<O: BufferContents + From<Matrix4<f32>>>(
    //     &mut self,
    //     buffer: &Subbuffer<[O]>,
    //     offset: usize,
    // ) -> Option<usize> {
    //     let obj_iter = self
    //         .shaders
    //         .iter_mut()
    //         .flat_map(|shader| shader.upload_pending_objects());
    //     write_to_storage_buffer(buffer, obj_iter, offset)
    // }

    /// bind draw calls to the given command buffer builder, be sure to call `update_object_buffer()` before hand
    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        object_index: &mut u32,
        sets: impl DescriptorSetsCollection + Clone,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        for pipeline_group in self.shaders.iter_mut() {
            pipeline_group.draw_objects(object_index, sets.clone(), command_builder);
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
