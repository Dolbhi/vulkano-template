use std::sync::Arc;

use cgmath::Matrix4;
use vulkano::{buffer::Subbuffer, descriptor_set::PersistentDescriptorSet};
use vulkano_template::shaders::basic::vs::{GPUCameraData, GPUObjectData};

use crate::render::renderer::Fence;

use super::render_object::RenderObject;

pub struct FrameData {
    pub fence: Option<Arc<Fence>>,

    camera_buffer: Subbuffer<GPUCameraData>,
    global_descriptor: Arc<PersistentDescriptorSet>,
    objects_buffer: Subbuffer<[GPUObjectData]>,
    object_descriptor: Arc<PersistentDescriptorSet>,
}

impl FrameData {
    pub fn new(
        camera_buffer: Subbuffer<GPUCameraData>,
        global_descriptor: Arc<PersistentDescriptorSet>,
        objects_buffer: Subbuffer<[GPUObjectData]>,
        object_descriptor: Arc<PersistentDescriptorSet>,
    ) -> Self {
        FrameData {
            fence: None,
            camera_buffer,
            global_descriptor,
            objects_buffer,
            object_descriptor,
        }
    }

    pub fn update_camera_data(&mut self, view: Matrix4<f32>, proj: Matrix4<f32>) {
        let mut cam_uniform_contents = self
            .camera_buffer
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to camera uniform buffer\n{}", e));
        cam_uniform_contents.view = view.into();
        cam_uniform_contents.proj = proj.into();
        cam_uniform_contents.view_proj = (proj * view).into();
    }

    pub fn update_objects_data(&mut self, render_objects: &Vec<RenderObject>) {
        let mut storage_buffer_contents = self
            .objects_buffer
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to camera uniform buffer\n{}", e));

        for i in 0..render_objects.len() {
            storage_buffer_contents[i].render_matrix =
                render_objects[i].get_transform_matrix().into();
        }
    }

    pub fn get_global_descriptor(&self) -> &Arc<PersistentDescriptorSet> {
        &self.global_descriptor
    }

    pub fn get_objects_descriptor(&self) -> &Arc<PersistentDescriptorSet> {
        &self.object_descriptor
    }
}
