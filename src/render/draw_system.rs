use std::{collections::HashMap, f32::consts::PI, iter::zip, sync::Arc, vec};

use vulkano::{
    buffer::BufferContents,
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{DescriptorSetWithOffsets, PersistentDescriptorSet},
    shader::EntryPoint,
};

use crate::{
    game_objects::Camera,
    shaders::draw::GPUGlobalData,
    vulkano_objects::{
        buffers::{create_global_descriptors, create_storage_buffers},
        pipeline::PipelineHandler,
    },
};

use super::{
    render_data::{
        frame_data::DrawBuffers,
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
    draw_buffers: Vec<DrawBuffers<O>>,
    descriptor_sets: Vec<Vec<DescriptorSetWithOffsets>>,
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
            draw_buffers: vec![],
            descriptor_sets: vec![],
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
        // TODO: Have object data type be generic
        let object_data = create_storage_buffers(
            &context.allocators,
            layout.set_layouts().get(1).unwrap().clone(),
            image_count,
            10000,
        );

        // create frame data
        for ((global_buffer, global_set), (storage_buffer, object_descriptor)) in
            zip(global_data, object_data)
        {
            let mut frame = DrawBuffers::new(global_buffer, storage_buffer);
            frame.update_scene_data(Some([0.2, 0.2, 0.2, 1.]), None, Some([0.9, 0.9, 0.6, 1.]));
            data.draw_buffers.push(frame);

            data.descriptor_sets
                .push(vec![global_set, object_descriptor.into()]);
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
                self.descriptor_sets[image_i].clone(),
                command_builder,
                &mut self.pending_objects,
            );
        }
    }
    /// write gpu data to respective buffers (currently auto rotates sunlight)
    pub fn upload_draw_data(
        &mut self,
        objects: impl Iterator<Item = &'a Arc<RenderObject<T>>>,
        camera_data: &Camera,
        aspect: f32,
        image_i: u32,
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
        let buffers = &mut self.draw_buffers[image_i as usize];
        buffers.update_objects_data(obj_iter);

        // update camera
        buffers.update_camera_data(
            camera_data.view_matrix(),
            camera_data.projection_matrix(aspect),
        );

        // update scene data
        let angle = PI / 4.;
        let cgmath::Vector3::<f32> { x, y, z } =
            cgmath::InnerSpace::normalize(cgmath::vec3(angle.sin(), -1., angle.cos()));
        buffers.update_scene_data(None, Some([x, y, z, 1.]), None);
    }

    // fn clear_unused_resource(&mut self) {
    //     for pipeline_group in self.pipelines.iter() {
    //         for material in pipeline_group.materials.iter() {
    //             if self.pending_objects
    //         }
    //     }
    // }
}
