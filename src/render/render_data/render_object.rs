use std::{collections::HashMap, sync::Arc, vec};

use cgmath::{Matrix4, Rad, SquareMatrix};
use vulkano::{
    command_buffer::AutoCommandBufferBuilder,
    descriptor_set::{DescriptorSetWithOffsets, PersistentDescriptorSet, WriteDescriptorSet},
    image::{sampler::Sampler, view::ImageView},
    pipeline::{PipelineBindPoint, PipelineLayout},
};

use crate::{
    vulkano_objects::{allocators::Allocators, buffers::Buffers, pipeline::PipelineHandler},
    VertexFull,
};

pub struct RenderObject<T: Clone> {
    pub mesh: Arc<Buffers<VertexFull>>,
    pub material_id: String,
    pub data: T,
}

impl<T: Clone> RenderObject<T> {
    pub fn get_data(&self) -> T {
        self.data.clone()
    }
}

impl RenderObject<Matrix4<f32>> {
    pub fn new(mesh: Arc<Buffers<VertexFull>>, material_id: String) -> Self {
        Self {
            mesh,
            material_id,
            // uniforms,
            data: Matrix4::identity(),
        }
    }

    pub fn update_transform(&mut self, position: [f32; 3], rotation: Rad<f32>) {
        let rotation = Matrix4::from_axis_angle([0., 1., 0.].into(), rotation);
        let translation = Matrix4::from_translation(position.into());

        self.data = translation * rotation;
    }

    // pub fn update_transform_axis(
    //     &mut self,
    //     position: [f32; 3],
    //     rotation: Rad<f32>,
    //     axis: [f32; 3],
    // ) {
    //     let rotation = Matrix4::from_axis_angle(axis.into(), rotation);
    //     let translation = Matrix4::from_translation(position.into());

    //     self.transform = translation * rotation;
    // }
}

pub struct PipelineGroup {
    pub pipeline: PipelineHandler,
    pub materials: Vec<MaterialGroup>,
}

impl PipelineGroup {
    pub fn new(pipeline: PipelineHandler) -> Self {
        PipelineGroup {
            pipeline,
            materials: vec![],
        }
    }

    /// NOTE: clears object vecs
    pub fn draw_objects<T, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &self,
        global_descriptor: &DescriptorSetWithOffsets,
        objects_descriptor: &DescriptorSetWithOffsets,
        command_builder: &mut AutoCommandBufferBuilder<T, A>,
        objects: &mut HashMap<String, Vec<Arc<RenderObject<Matrix4<f32>>>>>,
    ) {
        let mut index = 0;
        // bind pipeline
        command_builder
            .bind_pipeline_graphics(self.pipeline.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                vec![global_descriptor.clone(), objects_descriptor.clone()],
            )
            .unwrap();

        for material in &self.materials {
            // bind material sets
            material.bind_sets(&self.pipeline.layout(), command_builder);

            let mut last_mesh = None;
            let mut last_buffer_len = 0;
            for mesh in objects[&material.id].iter().map(|ro| &ro.mesh) {
                match last_mesh {
                    Some(old_mesh) if Arc::ptr_eq(old_mesh, &mesh) => {
                        // println!("Same mesh, skipping...");
                    }
                    _ => {
                        // bind mesh
                        let index_buffer = mesh.get_index();
                        let index_buffer_length = index_buffer.len();

                        command_builder
                            .bind_vertex_buffers(0, mesh.get_vertex())
                            .unwrap()
                            .bind_index_buffer(index_buffer)
                            .unwrap();

                        last_mesh = Some(&mesh);
                        last_buffer_len = index_buffer_length;
                    }
                }

                // draw
                command_builder
                    .draw_indexed(last_buffer_len as u32, 1, 0, 0, index as u32)
                    .unwrap();
                index += 1;
            }

            // clear render objects
            objects.get_mut(&material.id).unwrap().clear();
        }
    }

    // pub fn upload_objects<T: Iterator<Item = Arc<RenderObject>>>(
    //     &mut self,
    //     render_objects: &mut T,
    // ) {
    //     self.clear_objects();
    //     for render_object in render_objects {
    //         self.objects[&render_object.material_id].push(render_object.clone());
    //     }
    // }
    // fn clear_objects(&mut self) {
    //     // clear meshes
    //     for mat in &self.materials {
    //         self.objects[&mat.id].clear();
    //     }
    // }

    pub fn add_material(&mut self, id: String, set: Option<Arc<PersistentDescriptorSet>>) {
        let material = MaterialGroup {
            id,
            descriptor_set: set,
        };
        self.materials.push(material);
        // self.objects.insert(material.id, vec![]);
        // material
    }
    pub fn create_material_set(
        &self,
        allocators: &Allocators,
        index: usize,
        texture: Arc<ImageView>,
        sampler: Arc<Sampler>,
    ) -> Arc<PersistentDescriptorSet> {
        PersistentDescriptorSet::new(
            &allocators.descriptor_set,
            self.pipeline
                .layout()
                .set_layouts()
                .get(index)
                .unwrap()
                .clone(),
            [WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
            [],
        )
        .unwrap()
    }
}

pub struct MaterialGroup {
    pub id: String,
    pub descriptor_set: Option<Arc<PersistentDescriptorSet>>,
}
impl MaterialGroup {
    pub fn bind_sets<T, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &self,
        layout: &Arc<PipelineLayout>,
        command_builder: &mut AutoCommandBufferBuilder<T, A>,
    ) {
        if let Some(set) = &self.descriptor_set {
            command_builder
                .bind_descriptor_sets(
                    vulkano::pipeline::PipelineBindPoint::Graphics,
                    layout.clone(),
                    2,
                    set.clone(),
                )
                .unwrap();
        }
    }
}
