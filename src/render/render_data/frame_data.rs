use std::sync::Arc;

use crate::shaders::draw::{GPUCameraData, GPUSceneData};
use cgmath::Matrix4;
use vulkano::{
    buffer::{BufferContents, Subbuffer},
    descriptor_set::DescriptorSetWithOffsets,
};

use super::render_object::RenderObject;

pub struct FrameData<O: BufferContents> {
    camera_buffer: Subbuffer<GPUCameraData>,
    global_buffer: Subbuffer<GPUSceneData>,
    objects_buffer: Subbuffer<[O]>,
    pub descriptor_sets: Vec<DescriptorSetWithOffsets>,
}

impl<O: BufferContents> FrameData<O> {
    pub fn new(
        camera_buffer: Subbuffer<GPUCameraData>,
        global_buffer: Subbuffer<GPUSceneData>,
        objects_buffer: Subbuffer<[O]>,
        descriptor_sets: Vec<DescriptorSetWithOffsets>,
    ) -> Self {
        FrameData {
            camera_buffer,
            global_buffer,
            objects_buffer,
            descriptor_sets,
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

    pub fn update_scene_data(
        &mut self,
        ambient_color: Option<[f32; 4]>,
        sunlight_direction: Option<[f32; 4]>,
        sunlight_color: Option<[f32; 4]>,
    ) {
        let mut scene_uniform_contents = self
            .global_buffer
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to scene uniform buffer\n{}", e));

        if let Some(ambient) = ambient_color {
            scene_uniform_contents.ambient_color = ambient;
        }
        if let Some(direction) = sunlight_direction {
            scene_uniform_contents.sunlight_direction = direction;
            // scene_uniform_contents.sunlight_direction[1] *= -1.;
        }
        if let Some(color) = sunlight_color {
            scene_uniform_contents.sunlight_color = color;
        }
    }

    pub fn update_objects_data<'a, T>(
        &self,
        render_objects: impl Iterator<Item = &'a Arc<RenderObject<T>>>,
    ) where
        T: Into<O> + Clone + 'a,
    {
        let mut storage_buffer_contents = self
            .objects_buffer
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to object storage buffer\n{}", e));

        for (i, render_object) in render_objects.enumerate() {
            storage_buffer_contents[i] = render_object.get_data().into();
        }
    }
}
