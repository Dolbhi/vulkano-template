use std::sync::Arc;

use vulkano::{buffer::Subbuffer, descriptor_set::PersistentDescriptorSet};
use vulkano_template::shaders::basic::vs::CameraData;

use crate::render::renderer::Fence;

pub struct FrameData {
    pub fence: Option<Arc<Fence>>,
    camera_buffer: Subbuffer<CameraData>,
    global_descriptor: Arc<PersistentDescriptorSet>,
    // storage?
}

impl FrameData {
    pub fn update_camera_data(&mut self) {}

    pub fn get_global_descriptor(&self) -> &Arc<PersistentDescriptorSet> {
        &self.global_descriptor
    }
}
