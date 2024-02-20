// mod frame_data;
// use frame_data::FrameData;

use std::{collections::HashMap, sync::Arc, vec};

use vulkano::{
    buffer::{BufferContents, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        layout::DescriptorSetLayout, DescriptorSetsCollection, PersistentDescriptorSet,
    },
    render_pass::Subpass,
    shader::EntryPoint,
};

use crate::{
    render::{
        context::Context,
        render_data::{
            material::{MaterialID, PipelineGroup},
            render_object::RenderObject,
        },
    },
    vulkano_objects::{buffers::write_to_storage_buffer, pipeline::PipelineHandler},
};

/// Collection of pipelines and associated rendering data
///
/// All pipelines share sets 0 and 1, describing scene data and an array of object data (storage buffer) respectively
///
/// Materials can optionally add more sets, starting from set 2
///
/// T is type of renderobject
pub struct DrawSystem<T>
where
    T: Clone,
{
    pipelines: Vec<PipelineGroup>,
    pending_objects: HashMap<MaterialID, Vec<Arc<RenderObject<T>>>>,
}

impl<'a, T> DrawSystem<T>
where
    T: Clone + 'a,
{
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

        (
            DrawSystem {
                pipelines,
                pending_objects: HashMap::new(),
            },
            layouts,
        )
    }

    pub fn get_pipeline(&self, pipeline_index: usize) -> &PipelineGroup {
        &self.pipelines[pipeline_index]
    }
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

    pub fn add_material(
        &mut self,
        pipeline_index: usize,
        mat_id: impl Into<MaterialID>,
        set: Option<Arc<PersistentDescriptorSet>>,
    ) -> MaterialID {
        let id: MaterialID = mat_id.into();
        self.pipelines[pipeline_index].add_material(id.clone(), set);
        self.pending_objects.insert(id.clone(), vec![]);
        id
    }

    /// sort and write object data to given storage buffer
    pub fn upload_object_data<O: BufferContents + From<T>>(
        &mut self,
        // image_i: usize,
        objects: impl Iterator<Item = &'a Arc<RenderObject<T>>>,
        // global_data: impl Into<GPUGlobalData>,
        buffer: &Subbuffer<[O]>,
    ) {
        // sort renderobjects
        for object in objects {
            self.pending_objects
                .get_mut(&object.material_id)
                .unwrap()
                .push(object.clone());
        }
        // update renderobjects
        let obj_iter = self.pipelines.iter().flat_map(|pipeline| {
            pipeline
                .material_iter()
                .flat_map(|mat_id| self.pending_objects[mat_id].iter())
                .map(|ro| ro.get_data())
        });
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
        for pipeline_group in self.pipelines.iter() {
            pipeline_group.draw_objects(
                &mut object_index,
                sets.clone(), //self.frame_data[image_i].descriptor_sets.clone(),
                command_builder,
                &mut self.pending_objects,
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
