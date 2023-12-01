use std::{collections::HashMap, iter::zip, sync::Arc, vec};

use vulkano::{
    command_buffer::AutoCommandBufferBuilder, descriptor_set::PersistentDescriptorSet,
    shader::EntryPoint,
};

use crate::{
    game_objects::Camera,
    shaders::basic::{fs::GPUSceneData, vs::GPUCameraData},
    vulkano_objects::{
        buffers::{create_global_descriptors, create_storage_buffers},
        pipeline::PipelineHandler,
    },
};

use self::{
    frame_data::FrameData,
    render_object::{PipelineGroup, RenderObject},
};

use super::renderer::Renderer;

pub mod frame_data;
pub mod material;
pub mod mesh;
pub mod render_object;
pub mod texture;

/// Collection of all data needed for rendering
pub struct RenderData {
    pipelines: Vec<PipelineGroup>,
    frames: Vec<FrameData>,
    pending_objects: HashMap<String, Vec<Arc<RenderObject<cgmath::Matrix4<f32>>>>>,
}

impl RenderData {
    /// creates a pipelines collection using 1 pipeline that future added pipelines must match
    pub fn new(context: &Renderer, vs: EntryPoint, fs: EntryPoint) -> Self {
        // initialize
        let mut data = RenderData {
            pipelines: vec![],
            frames: vec![],
            pending_objects: HashMap::new(),
        };
        data.add_pipeline(&context, vs, fs);

        let layout = data.pipelines[0].pipeline.layout();
        let image_count = context.swapchain.image_count() as usize;

        // create buffers and descriptors
        let global_data = create_global_descriptors::<GPUCameraData, GPUSceneData>(
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
        for ((cam_buffer, scene_buffer, global_set), (storage_buffer, object_descriptor)) in
            zip(global_data, object_data)
        {
            let mut frame = FrameData::new(
                cam_buffer,
                scene_buffer,
                global_set,
                storage_buffer,
                object_descriptor.into(),
            );
            frame.update_scene_data(Some([0.2, 0.2, 0.2, 1.]), None, Some([0.9, 0.9, 0.6, 1.]));
            data.frames.push(frame);
        }

        data
    }

    pub fn add_pipeline(&mut self, context: &Renderer, vs: EntryPoint, fs: EntryPoint) -> usize {
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
        mat_id: String,
        set: Option<Arc<PersistentDescriptorSet>>,
    ) {
        self.pipelines[pipeline_index].add_material(mat_id.clone(), set);
        self.pending_objects.insert(mat_id, vec![]);
    }

    /// bind draw calls to the given command buffer builder, be sure to call `upload_draw_data()` before hand
    pub fn render<T, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &mut self,
        image_i: usize,
        command_builder: &mut AutoCommandBufferBuilder<T, A>,
    ) {
        let frame = &self.frames[image_i];
        for pipeline_group in self.pipelines.iter() {
            pipeline_group.draw_objects(
                &frame.global_descriptor,
                &frame.objects_descriptor,
                command_builder,
                &mut self.pending_objects,
            );
        }
    }
    /// write gpu data to respective buffers (currently auto rotates sunlight)
    pub fn upload_draw_data<'a>(
        &mut self,
        objects: impl Iterator<Item = &'a Arc<RenderObject<cgmath::Matrix4<f32>>>>,
        camera_data: &Camera,
        aspect: f32,
        image_i: u32,
        total_seconds: f32,
    ) {
        // update object data
        // match Arc::get_mut(&mut self.render_objects[0]) {
        //     Some(obj) => {
        //         obj.update_transform([0., 0., 0.], cgmath::Rad(total_seconds));
        //     }
        //     None => {
        //         panic!("Unable to update render object");
        //     }
        // }
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
                .materials
                .iter()
                .flat_map(|mat| self.pending_objects[&mat.id].iter())
        });
        let frame = &mut self.frames[image_i as usize];
        frame.update_objects_data(obj_iter);

        // update camera
        frame.update_camera_data(
            camera_data.view_matrix(),
            camera_data.projection_matrix(aspect),
        );

        // update scene data
        let angle = total_seconds / 2.;
        let cgmath::Vector3::<f32> { x, y, z } =
            cgmath::InnerSpace::normalize(cgmath::vec3(angle.sin(), -1., angle.cos()));
        frame.update_scene_data(None, Some([x, y, z, 1.]), None);
    }

    // fn clear_unused_resource(&mut self) {
    //     for pipeline_group in self.pipelines.iter() {
    //         for material in pipeline_group.materials.iter() {
    //             if self.pending_objects
    //         }
    //     }
    // }
}