use std::{
    fmt::Display,
    sync::{Arc, Mutex},
    vec,
};

use cgmath::Matrix4;
use vulkano::{
    command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder},
    descriptor_set::{DescriptorSetsCollection, PersistentDescriptorSet},
    pipeline::{PipelineBindPoint, PipelineLayout},
};

use crate::{
    render::resource_manager::MaterialID,
    vulkano_objects::{buffers::MeshBuffers, pipeline::PipelineHandler},
    VertexFull,
};

/// Collection of materials using the same pipeline
///
/// T: Type of additional data in render object
pub struct Shader<T: Clone> {
    id: MaterialID,
    pub pipeline: PipelineHandler,
    materials: Vec<Material<T>>,
}

/// Arc Mutex storing renderobject data to be uploaded
pub type RenderSubmit<T> = Arc<Mutex<Vec<(Arc<MeshBuffers<VertexFull>>, Matrix4<f32>, T)>>>;

struct Material<T: Clone> {
    pub descriptor_set: Option<Arc<PersistentDescriptorSet>>,
    pending_objects: RenderSubmit<T>,
    pending_meshes: Vec<Arc<MeshBuffers<VertexFull>>>,
}

impl<T: Clone> Display for Shader<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Shader {:?} with {} material(s)",
            self.id,
            self.materials.len()
        ))
    }
}

impl<T: Clone> Shader<T> {
    pub fn new(id: MaterialID, pipeline: PipelineHandler) -> Self {
        Shader {
            id,
            pipeline,
            materials: vec![],
        }
    }

    pub fn get_id(&self) -> MaterialID {
        self.id
    }

    /// Add draw calls of each object in each material of this pipeline
    ///
    /// NOTE: clears object vecs
    ///
    /// *  `objects` - Hashmap of object vecs with their material as the key
    pub fn draw_objects<C, A: CommandBufferAllocator>(
        &mut self,
        object_index: &mut u32,
        descriptor_sets: impl DescriptorSetsCollection,
        command_builder: &mut AutoCommandBufferBuilder<C, A>,
        // objects: &mut HashMap<MaterialID, Vec<Arc<RenderObject<T>>>>,
    ) {
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

        for material in &mut self.materials {
            // bind material sets
            material.bind_sets(self.pipeline.layout(), command_builder);

            // Draw objects with the same mesh in a single instanced draw call
            let mut last_mesh = None;
            let mut last_buffer_len = 0;
            let mut instance_count = 0;
            for mesh in material.pending_meshes.iter() {
                match last_mesh {
                    Some(old_mesh) if Arc::ptr_eq(old_mesh, mesh) => {
                        // println!("Same mesh, skipping...");
                    }
                    Some(_) => {
                        // New mesh, draw old mesh and bind new one

                        // draw
                        command_builder
                            .draw_indexed(
                                last_buffer_len as u32,
                                instance_count,
                                0,
                                0,
                                *object_index,
                            )
                            .unwrap();
                        *object_index += instance_count;
                        instance_count = 0;

                        // bind mesh
                        let index_buffer = mesh.get_index();
                        let index_buffer_length = index_buffer.len();

                        command_builder
                            .bind_vertex_buffers(0, mesh.get_vertex())
                            .unwrap()
                            .bind_index_buffer(index_buffer)
                            .unwrap();

                        last_mesh = Some(mesh);
                        last_buffer_len = index_buffer_length;
                    }
                    _ => {
                        // First mesh, bind for later drawing

                        // bind mesh
                        let index_buffer = mesh.get_index();
                        let index_buffer_length = index_buffer.len();

                        command_builder
                            .bind_vertex_buffers(0, mesh.get_vertex())
                            .unwrap()
                            .bind_index_buffer(index_buffer)
                            .unwrap();

                        last_mesh = Some(mesh);
                        last_buffer_len = index_buffer_length;
                    }
                }
                instance_count += 1;
            }
            // Draw last mesh
            if instance_count > 0 {
                // draw
                command_builder
                    .draw_indexed(last_buffer_len as u32, instance_count, 0, 0, *object_index)
                    .unwrap();
                *object_index += instance_count;
            }

            // clear render objects
            material.pending_meshes.clear();
        }
    }

    /// creates a material and returns a mutex vec for submitting render objects
    pub fn add_material(&mut self, set: Option<Arc<PersistentDescriptorSet>>) -> RenderSubmit<T> {
        let pending_objects = Arc::new(Mutex::new(vec![]));
        let material = Material {
            descriptor_set: set,
            pending_objects: pending_objects.clone(),
            pending_meshes: vec![],
        };
        self.materials.push(material);
        pending_objects
    }

    // pub fn create_material_set(
    //     &self,
    //     allocators: &Allocators,
    //     descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
    // ) -> Arc<PersistentDescriptorSet> {
    //     PersistentDescriptorSet::new(
    //         &allocators.descriptor_set,
    //         self.pipeline.layout().set_layouts().get(2).unwrap().clone(),
    //         descriptor_writes,
    //         [],
    //     )
    //     .unwrap()
    // }

    /// returns all pending object data in an iterator and queue meshes for rendering
    pub fn upload_pending_objects(&mut self) -> impl Iterator<Item = (Matrix4<f32>, T)> + '_ {
        self.materials.iter_mut().flat_map(|mat| {
            let mut objs = mat.pending_objects.lock().unwrap();
            std::mem::take(&mut *objs)
                .into_iter()
                .map(|(mesh, model, data)| {
                    mat.pending_meshes.push(mesh);
                    (model, data)
                })
            // .collect::<Vec<Matrix4<f32>>>()
        })
    }
}

impl<T: Clone> Material<T> {
    // bind material sets starting from set 2
    fn bind_sets<L, A: vulkano::command_buffer::allocator::CommandBufferAllocator>(
        &self,
        layout: &Arc<PipelineLayout>,
        command_builder: &mut AutoCommandBufferBuilder<L, A>,
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
