// mod frame_data;
// use frame_data::FrameData;

use std::{sync::Arc, vec};

use cgmath::Matrix4;
use vulkano::{
    buffer::{BufferContents, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{layout::DescriptorSetLayout, DescriptorSetsCollection},
    render_pass::Subpass,
    shader::EntryPoint,
};

use crate::{
    render::{context::Context, render_data::material::PipelineGroup},
    vulkano_objects::{
        buffers::{write_to_storage_buffer, Buffers},
        pipeline::PipelineHandler,
    },
    VertexFull,
};

/// Collection of pipelines and associated rendering data
///
/// All pipelines share sets 0 and 1, describing scene data and an array of object data (storage buffer) respectively
///
/// Materials can optionally add more sets, starting from set 2
pub struct DrawSystem {
    pub pipelines: Vec<PipelineGroup>,
}

impl<'a> DrawSystem {
    /// creates from a collection of shader entry points
    pub fn new(
        context: &Context,
        subpass: &Subpass,
        shaders: impl IntoIterator<Item = (EntryPoint, EntryPoint)>,
    ) -> (Self, [Arc<DescriptorSetLayout>; 2]) {
        let pipelines: Vec<PipelineGroup> = shaders
            .into_iter()
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
            .map(PipelineGroup::new)
            .collect();

        let layouts = pipelines[0].pipeline.layout().set_layouts();
        let layouts = [layouts[0].clone(), layouts[1].clone()];

        (DrawSystem { pipelines }, layouts)
    }

    // pub fn get_pipeline(&self, pipeline_index: usize) -> &PipelineGroup {
    //     &self.pipelines[pipeline_index]
    // }
    /// Recreate all pipelines with any changes in viewport
    ///
    /// See also: [recreate_pipeline](PipelineHandler::recreate_pipeline)
    pub fn recreate_pipelines(&mut self, context: &Context) {
        for pipeline in self.pipelines.iter_mut() {
            pipeline
                .pipeline
                .recreate_pipeline(context.device.clone(), context.viewport.clone());
        }
    }

    // pub fn add_material(
    //     &mut self,
    //     pipeline_index: usize,
    //     mat_id: impl Into<MaterialID>,
    //     set: Option<Arc<PersistentDescriptorSet>>,
    // ) -> RenderSubmit {
    //     let id: MaterialID = mat_id.into();
    //     self.pipelines[pipeline_index].add_material(id.clone(), set)
    //     // self.pending_objects.insert(id.clone(), vec![]);
    //     // id
    // }

    /// sort and write object data to given storage buffer (must be called before rendering)
    pub fn update_object_buffer<O: BufferContents + From<Matrix4<f32>>>(
        &mut self,
        buffer: &Subbuffer<[O]>,
    ) {
        // // sort renderobjects
        // for object in objects {
        //     self.pending_objects
        //         .get_mut(&object.material_id)
        //         .unwrap()
        //         .push(object.clone());
        // }
        // update renderobjects
        let obj_iter = self
            .pipelines
            .iter_mut()
            .flat_map(|pipeline| pipeline.upload_pending_objects());
        write_to_storage_buffer(buffer, obj_iter);
    }
    /// bind draw calls to the given command buffer builder, be sure to call `upload_draw_data()` before hand
    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        // image_i: usize,
        sets: impl DescriptorSetsCollection + Clone,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        let mut object_index = 0;
        for pipeline_group in self.pipelines.iter_mut() {
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
