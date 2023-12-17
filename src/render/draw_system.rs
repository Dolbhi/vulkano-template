mod frame_data;
use frame_data::FrameData;

use std::{collections::HashMap, iter::zip, sync::Arc, vec};

use vulkano::{
    buffer::BufferContents, command_buffer::AutoCommandBufferBuilder,
    descriptor_set::PersistentDescriptorSet, shader::EntryPoint,
};

use crate::{
    shaders::draw::GPUGlobalData,
    vulkano_objects::{
        buffers::{create_global_descriptors, create_storage_buffers},
        pipeline::PipelineHandler,
    },
};

use super::{
    render_data::{
        material::{MaterialID, PipelineGroup},
        render_object::RenderObject,
    },
    renderer::Renderer,
};

/// Collection of all data needed for rendering
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
    /// creates a pipelines collection using 1 pipeline that future added pipelines must match
    pub fn new(
        context: &Renderer,
        shaders: impl IntoIterator<Item = (EntryPoint, EntryPoint)>,
    ) -> Self {
        // initialize
        let mut data = DrawSystem {
            pipelines: vec![],
            frame_data: vec![],
            pending_objects: HashMap::new(),
        };
        for (vs, fs) in shaders {
            data.add_pipeline(&context, vs, fs);
        }

        let layout = data.pipelines[0].pipeline.layout();
        let image_count = context.swapchain.image_count() as usize;

        // create buffers and descriptors
        let global_data = create_global_descriptors::<GPUGlobalData>(
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
        for ((global_buffer, global_set), (objects_buffer, object_descriptor)) in
            zip(global_data, object_data)
        {
            let frame = FrameData {
                global_buffer,
                objects_buffer,
                descriptor_sets: vec![global_set, object_descriptor.into()],
            };
            data.frame_data.push(frame);
        }

        data
    }
    fn add_pipeline(&mut self, context: &Renderer, vs: EntryPoint, fs: EntryPoint) -> usize {
        let pipeline = PipelineHandler::new(
            context.device.clone(),
            vs,
            fs,
            context.viewport.clone(),
            context.render_pass.clone(),
            [(0, 0)],
            crate::vulkano_objects::pipeline::PipelineType::Drawing,
        );
        self.pipelines.push(PipelineGroup::new(pipeline));
        self.pipelines.len() - 1
    }

    pub fn get_pipeline(&self, pipeline_index: usize) -> &PipelineGroup {
        &self.pipelines[pipeline_index]
    }
    pub fn recreate_pipelines(&mut self, context: &Renderer) {
        for pipeline in self.pipelines.iter_mut() {
            pipeline.pipeline.recreate_pipeline(
                context.device.clone(),
                context.render_pass.clone(),
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
    /// write gpu data to respective buffers (currently auto rotates sunlight)
    pub fn upload_draw_data(
        &mut self,
        image_i: u32,
        objects: impl Iterator<Item = &'a Arc<RenderObject<T>>>,
        proj: impl Into<[[f32; 4]; 4]>,
        view: impl Into<[[f32; 4]; 4]>,
        proj_view: impl Into<[[f32; 4]; 4]>,
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
        let buffers = &mut self.frame_data[image_i as usize];
        buffers.update_objects_data(obj_iter);

        // update camera
        buffers.update_global_data(GPUGlobalData {
            proj: proj.into(),
            view: view.into(),
            view_proj: proj_view.into(),
        });
    }

    // fn clear_unused_resource(&mut self) {
    //     for pipeline_group in self.pipelines.iter() {
    //         for material in pipeline_group.materials.iter() {
    //             if self.pending_objects
    //         }
    //     }
    // }
}
