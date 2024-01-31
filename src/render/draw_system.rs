// mod frame_data;
// use frame_data::FrameData;

use std::{collections::HashMap, sync::Arc, vec};

use vulkano::{
    buffer::{BufferContents, Subbuffer},
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{
        layout::DescriptorSetLayout, DescriptorSetWithOffsets, PersistentDescriptorSet,
    },
    render_pass::RenderPass,
    shader::EntryPoint,
};

use crate::vulkano_objects::{buffers::write_to_storage_buffer, pipeline::PipelineHandler};

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
pub struct DrawSystem<T>
where
    // O: BufferContents + From<T>,
    T: Clone,
{
    pipelines: Vec<PipelineGroup>,
    // frame_data: Vec<FrameData<O>>,
    pending_objects: HashMap<MaterialID, Vec<Arc<RenderObject<T>>>>,
}

impl<'a, T> DrawSystem<T>
where
    T: Clone + 'a,
{
    /// creates from a collection of shader entry points
    pub fn new(
        context: &Context,
        render_pass: &Arc<RenderPass>,
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
                    render_pass.clone(),
                    [], // [(0, 0)],
                    crate::vulkano_objects::pipeline::PipelineType::Drawing,
                )
            })
            .map(PipelineGroup::new)
            .collect();

        let layouts = pipelines[0].pipeline.layout().set_layouts();
        let layouts = [layouts[0].clone(), layouts[1].clone()];

        // // create buffers and descriptors
        // let global_data = create_dynamic_buffers::<GPUGlobalData>(
        //     &context.allocators,
        //     &context.device,
        //     layout.set_layouts().get(0).unwrap().clone(),
        //     image_count,
        // );
        // let object_data = create_storage_buffers(
        //     &context.allocators,
        //     layout.set_layouts().get(1).unwrap().clone(),
        //     image_count,
        //     10000,
        // );

        // // create frame data
        // let frame_data = zip(global_data, object_data)
        //     .map(
        //         |((global_buffer, global_set), (objects_buffer, object_descriptor))| FrameData {
        //             global_buffer,
        //             objects_buffer,
        //             descriptor_sets: vec![global_set, object_descriptor.into()],
        //         },
        //     )
        //     .collect();

        (
            DrawSystem {
                pipelines,
                // frame_data,
                pending_objects: HashMap::new(),
            },
            layouts,
        )
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
        // let buffers = &mut self.frame_data[image_i];
        // buffers.update_objects_data(obj_iter);

        // // update camera
        // buffers.update_global_data(global_data);
    }
    /// bind draw calls to the given command buffer builder, be sure to call `upload_draw_data()` before hand
    pub fn render<P, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        // image_i: usize,
        global_set: DescriptorSetWithOffsets,
        object_set: DescriptorSetWithOffsets,
        command_builder: &mut AutoCommandBufferBuilder<P, A>,
    ) {
        let mut object_index = 0;
        let sets = vec![global_set, object_set];
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

// pub struct FrameData<O: BufferContents> {
//     global_buffer: Subbuffer<GPUGlobalData>,
//     objects_buffer: Subbuffer<[O]>,
//     descriptor_sets: Vec<DescriptorSetWithOffsets>,
// }

// impl<O: BufferContents> FrameData<O> {
//     pub fn update_global_data(&mut self, data: impl Into<GPUGlobalData>) {
//         write_to_buffer(&self.global_buffer, data);
//     }

//     pub fn update_objects_data<'a, T>(
//         &self,
//         render_objects: impl Iterator<Item = &'a Arc<RenderObject<T>>>,
//     ) where
//         T: Into<O> + Clone + 'a,
//     {
//         write_to_storage_buffer(&self.objects_buffer, render_objects.map(|ro| ro.get_data()));
//     }
// }
