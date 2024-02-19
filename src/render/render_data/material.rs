use std::{collections::HashMap, sync::Arc};

use vulkano::{
    command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder},
    descriptor_set::{DescriptorSetsCollection, PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::{PipelineBindPoint, PipelineLayout},
};

use crate::{
    vulkano_objects::{allocators::Allocators, pipeline::PipelineHandler},
    VertexFull,
};

use super::render_object::RenderObject;

pub struct PipelineGroup {
    pub pipeline: PipelineHandler<VertexFull>,
    materials: Vec<Material>,
}

impl PipelineGroup {
    pub fn new(pipeline: PipelineHandler<VertexFull>) -> Self {
        PipelineGroup {
            pipeline,
            materials: vec![],
        }
    }

    /// NOTE: clears object vecs
    pub fn draw_objects<T: Clone, C, A: CommandBufferAllocator>(
        &self,
        object_index: &mut u32,
        descriptor_sets: impl DescriptorSetsCollection,
        command_builder: &mut AutoCommandBufferBuilder<C, A>,
        objects: &mut HashMap<MaterialID, Vec<Arc<RenderObject<T>>>>,
    ) {
        // let mut index = 0;
        // bind pipeline
        command_builder
            .bind_pipeline_graphics(self.pipeline.pipeline.clone())
            .unwrap()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_sets,
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
                    .draw_indexed(last_buffer_len as u32, 1, 0, 0, *object_index)
                    .unwrap();
                *object_index += 1;
            }

            // clear render objects
            objects.get_mut(&material.id).unwrap().clear();
        }
    }

    pub fn add_material(&mut self, id: MaterialID, set: Option<Arc<PersistentDescriptorSet>>) {
        let material = Material {
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
        descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
    ) -> Arc<PersistentDescriptorSet> {
        PersistentDescriptorSet::new(
            &allocators.descriptor_set,
            self.pipeline
                .layout()
                .set_layouts()
                .get(index)
                .unwrap()
                .clone(),
            descriptor_writes,
            [],
        )
        .unwrap()
    }
    pub fn material_iter(&self) -> impl Iterator<Item = &MaterialID> {
        self.materials.iter().map(|mat| &mat.id)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct MaterialID(pub String);
impl From<String> for MaterialID {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for MaterialID {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

struct Material {
    pub id: MaterialID,
    pub descriptor_set: Option<Arc<PersistentDescriptorSet>>,
}
impl Material {
    fn bind_sets<T, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
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
