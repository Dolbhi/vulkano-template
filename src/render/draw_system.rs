mod frame_data;
use frame_data::FrameData;

use std::{collections::HashMap, iter::zip, sync::Arc, vec};

use vulkano::{
    buffer::BufferContents, command_buffer::AutoCommandBufferBuilder,
    descriptor_set::PersistentDescriptorSet, render_pass::RenderPass, shader::EntryPoint,
};

use crate::{
    shaders::draw::GPUGlobalData,
    vulkano_objects::{
        buffers::{create_dynamic_buffers, create_storage_buffers},
        pipeline::PipelineHandler,
    },
};

use super::{
    context::Context,
    render_data::{
        material::{MaterialID, PipelineGroup},
        render_object::RenderObject,
    },
};

/// Collection of pipelines and associated rendering data
///
/// All pipelines share sets 1 and 2, describing scene data and an array of object data (storage buffer) respectively
///
/// Materials can optionally add more sets
pub struct DrawSystem<O, T>
where
    O: BufferContents + From<T>,
    T: Clone,
{
    pipelines: Vec<PipelineGroup>,
    frame_data: Vec<FrameData<O>>,
    pending_objects: HashMap<MaterialID, Vec<Arc<RenderObject<T>>>>,
}

impl<'a, O, T> DrawSystem<O, T>
where
    O: BufferContents + From<T>,
    T: Clone + 'a,
{
    /// creates from a collection of shader entry points
    pub fn new(
        context: &Context,
        render_pass: &Arc<RenderPass>,
        shaders: impl IntoIterator<Item = (EntryPoint, EntryPoint)>,
    ) -> Self {
        let pipelines: Vec<PipelineGroup> = shaders
            .into_iter()
            .map(|(vs, fs)| {
                PipelineHandler::new(
                    context.device.clone(),
                    vs,
                    fs,
                    context.viewport.clone(),
                    render_pass.clone(),
                    [(0, 0)],
                    crate::vulkano_objects::pipeline::PipelineType::Drawing,
                )
            })
            .map(PipelineGroup::new)
            .collect();

        let layout = pipelines[0].pipeline.layout();
        let image_count = context.get_image_count();

        // create buffers and descriptors
        let global_data = create_dynamic_buffers::<GPUGlobalData>(
            &context.allocators,
            &context.device,
            layout.set_layouts().get(0).unwrap().clone(),
            image_count,
        );
        let object_data = create_storage_buffers(
            &context.allocators,
            layout.set_layouts().get(1).unwrap().clone(),
            image_count,
            10000,
        );

        // create frame data
        let frame_data = zip(global_data, object_data)
            .map(
                |((global_buffer, global_set), (objects_buffer, object_descriptor))| FrameData {
                    global_buffer,
                    objects_buffer,
                    descriptor_sets: vec![global_set, object_descriptor.into()],
                },
            )
            .collect();

        DrawSystem {
            pipelines,
            frame_data,
            pending_objects: HashMap::new(),
        }
    }

    pub fn get_pipeline(&self, pipeline_index: usize) -> &PipelineGroup {
        &self.pipelines[pipeline_index]
    }
    pub fn recreate_pipelines(&mut self, context: &Context, render_pass: &Arc<RenderPass>) {
        for pipeline in self.pipelines.iter_mut() {
            pipeline.pipeline.recreate_pipeline(
                context.device.clone(),
                render_pass.clone(),
                context.viewport.clone(),
            );
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

    /// write gpu data to respective buffers (currently auto rotates sunlight)
    pub fn upload_draw_data(
        &mut self,
        image_i: usize,
        objects: impl Iterator<Item = &'a Arc<RenderObject<T>>>,
        global_data: impl Into<GPUGlobalData>,
        // proj: impl Into<[[f32; 4]; 4]>,
        // view: impl Into<[[f32; 4]; 4]>,
        // proj_view: impl Into<[[f32; 4]; 4]>,
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
        });
        let buffers = &mut self.frame_data[image_i];
        buffers.update_objects_data(obj_iter);

        // update camera
        buffers.update_global_data(global_data);
    }
    /// bind draw calls to the given command buffer builder, be sure to call `upload_draw_data()` before hand
    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        image_i: usize,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        let mut object_index = 0;
        for pipeline_group in self.pipelines.iter() {
            pipeline_group.draw_objects(
                &mut object_index,
                self.frame_data[image_i].descriptor_sets.clone(),
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
