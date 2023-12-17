use std::sync::Arc;

use crate::{render::RenderObject, shaders::draw::GPUGlobalData};
use vulkano::{
    buffer::{BufferContents, Subbuffer},
    descriptor_set::DescriptorSetWithOffsets,
};

pub struct FrameData<O: BufferContents> {
    pub global_buffer: Subbuffer<GPUGlobalData>,
    pub objects_buffer: Subbuffer<[O]>,
    pub descriptor_sets: Vec<DescriptorSetWithOffsets>,
}

impl<O: BufferContents> FrameData<O> {
    pub fn update_global_data(&mut self, data: impl Into<GPUGlobalData>) {
        let mut uniform_contents = self
            .global_buffer
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to camera uniform buffer\n{}", e));
        *uniform_contents = data.into();
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
