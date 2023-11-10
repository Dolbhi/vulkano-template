use std::sync::Arc;

use cgmath::{Matrix4, Rad, SquareMatrix};

use vulkano::{buffer::BufferContents, descriptor_set::PersistentDescriptorSet};
use vulkano_template::{shaders::basic::vs::GPUObjectData, vulkano_objects::buffers::Uniform};

pub struct RenderObject<U: BufferContents + Clone> {
    pub mesh_id: String,
    pub material_id: String,
    uniforms: Vec<Uniform<U>>,
    transform: Matrix4<f32>,
}

impl<U: BufferContents + Clone> RenderObject<U> {
    pub fn new(mesh_id: String, material_id: String, uniforms: Vec<Uniform<U>>) -> Self {
        Self {
            mesh_id,
            material_id,
            uniforms,
            transform: Matrix4::identity(),
        }
    }

    // pub fn get_uniforms(&self) -> &Vec<Uniform<U>> {
    //     &self.uniforms
    // }

    pub fn clone_descriptor(&self, index: usize) -> Arc<PersistentDescriptorSet> {
        self.uniforms[index].1.clone()
    }

    pub fn update_transform(&mut self, position: [f32; 3], rotation: Rad<f32>) {
        let rotation = Matrix4::from_axis_angle([0., 1., 0.].into(), rotation);
        let translation = Matrix4::from_translation(position.into());

        self.transform = translation * rotation;
    }
}

impl RenderObject<GPUObjectData> {
    pub fn update_uniform(&self, index: u32) {
        let mut uniform_content = self.uniforms[index as usize]
            .0
            .write()
            .unwrap_or_else(|e| panic!("Failed to write to uniform buffer\n{}", e));

        uniform_content.render_matrix = self.transform.into();
    }
}
