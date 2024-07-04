//! Contain functions for creating various types of buffers and descriptor sets
//! Reusable for multiple renderers

use std::{mem::size_of, sync::Arc};

use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage, CopyBufferInfo,
        PrimaryCommandBufferAbstract,
    },
    descriptor_set::{
        layout::DescriptorSetLayout, DescriptorBufferInfo, DescriptorSet, DescriptorSetWithOffsets,
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::graphics::vertex_input::Vertex,
    sync::{future::NowFuture, AccessConflict, GpuFuture, HostAccessError},
    DeviceSize,
};

use super::allocators::Allocators;

pub type Uniform<U> = (Subbuffer<U>, Arc<PersistentDescriptorSet>);

/// Buffers for vertcies and indecies, essentially a struct containing mesh data
#[derive(Debug)]
pub struct MeshBuffers<V: Vertex + BufferContents> {
    pub vertex: Subbuffer<[V]>,
    pub index: Subbuffer<[u32]>,
    // pub uniforms: Vec<Uniform<U>>,
}

impl<V: Vertex + BufferContents> MeshBuffers<V> {
    /// Creates device local vertex and index buffers of specified model
    pub fn initialize_device_local(
        allocators: &Allocators,
        transfer_queue: Arc<Queue>,
        vertices: Vec<V>,
        indices: Vec<u32>,
    ) -> Self {
        let (vertex, vertex_future) = create_device_local_buffer(
            allocators,
            transfer_queue.clone(),
            vertices,
            BufferUsage::VERTEX_BUFFER,
        );
        let (index, index_future) = create_device_local_buffer(
            allocators,
            transfer_queue.clone(),
            indices,
            BufferUsage::INDEX_BUFFER,
        );

        let fence = vertex_future
            .join(index_future)
            .then_signal_fence_and_flush()
            .unwrap();
        fence.wait(None).unwrap();

        Self { vertex, index }
    }

    pub fn get_vertex(&self) -> Subbuffer<[V]> {
        self.vertex.clone()
    }

    pub fn get_index(&self) -> Subbuffer<[u32]> {
        self.index.clone()
    }

    pub fn index_len(&self) -> u64 {
        self.index.len()
    }
}

// creates a uniform device local buffer
pub fn create_material_buffer<T: BufferContents>(
    allocators: &Allocators,
    data: T,
    usage: BufferUsage,
) -> Subbuffer<T> {
    Buffer::from_data(
        allocators.memory.clone(),
        BufferCreateInfo {
            usage: usage | BufferUsage::UNIFORM_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        data,
    )
    .unwrap()
}

/// returns a device only buffer and a future that copies data to it
pub fn create_device_local_buffer<T: BufferContents>(
    allocators: &Allocators,
    queue: Arc<Queue>,
    values: Vec<T>,
    usage: BufferUsage,
) -> (Subbuffer<[T]>, CommandBufferExecFuture<NowFuture>) {
    let buffer = Buffer::new_slice(
        allocators.memory.clone(),
        BufferCreateInfo {
            usage: usage | BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        values.len() as DeviceSize,
    )
    .unwrap();

    let staging_buffer = Buffer::from_iter(
        allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        values,
    )
    .unwrap();

    let mut builder = AutoCommandBufferBuilder::primary(
        &allocators.command_buffer,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    builder
        .copy_buffer(CopyBufferInfo::buffers(staging_buffer, buffer.clone()))
        .unwrap();

    let future = builder.build().unwrap().execute(queue).unwrap();

    (buffer, future)
}

// /// returns uniform buffers with corresponding descriptor sets for interfacing
// pub fn create_cpu_accessible_uniforms<U: BufferContents + Clone>(
//     allocators: &Allocators,
//     descriptor_set_layout: Arc<DescriptorSetLayout>,
//     buffer_count: usize,
//     initial_uniform: U,
// ) -> Vec<Uniform<U>> {
//     (0..buffer_count)
//         .map(|_| {
//             let buffer = Buffer::from_data(
//                 allocators.memory.clone(),
//                 BufferCreateInfo {
//                     usage: BufferUsage::UNIFORM_BUFFER,
//                     ..Default::default()
//                 },
//                 AllocationCreateInfo {
//                     memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
//                         | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
//                     ..Default::default()
//                 },
//                 initial_uniform.clone(),
//             )
//             .unwrap();
//
//             // descriptor set is how we interface data between the buffer and the pipeline
//             let descriptor_set = PersistentDescriptorSet::new(
//                 &allocators.descriptor_set,
//                 descriptor_set_layout.clone(),
//                 [WriteDescriptorSet::buffer(0, buffer.clone())],
//                 [],
//             )
//             .unwrap();
//
//             (buffer, descriptor_set)
//         })
//         .collect()
// }

/// Creates a dynamic buffer to store global data, and a descriptor set for those buffers to be used with offsets
///
/// Returns vec of tuples containing the subbuffers and the descriptor set with the offset for that frame
pub fn create_dynamic_buffers<C: BufferContents>(
    allocators: &Allocators,
    device: &Arc<Device>,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    buffer_count: usize,
) -> Vec<(Subbuffer<C>, DescriptorSetWithOffsets)> {
    let content_size = size_of::<C>() as DeviceSize;

    let align = {
        let min_dynamic_align = device
            .physical_device()
            .properties()
            .min_uniform_buffer_offset_alignment
            .as_devicesize();

        // Round size up to the next multiple of align.
        (content_size + min_dynamic_align - 1) & !(min_dynamic_align - 1)
    };
    let dynamic_buffer = {
        let data_size = buffer_count as u64 * align;

        Buffer::new_slice::<u8>(
            allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data_size,
        )
        .unwrap()
    };

    // descriptor set is how we interface data between the buffer and the pipeline
    let descriptor_set = PersistentDescriptorSet::new(
        &allocators.descriptor_set,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::buffer_with_range(
            0,
            DescriptorBufferInfo {
                buffer: dynamic_buffer.clone(),
                range: 0..content_size,
            },
        )],
        [],
    )
    .unwrap();

    (0..buffer_count as DeviceSize)
        .map(|i| {
            let start = i * align;
            let end = start + content_size;

            let buffer = dynamic_buffer.clone().slice(start..end).reinterpret::<C>();

            let offset = (align * i) as u32;
            let set = descriptor_set.clone().offsets([offset]);

            (buffer, set)
        })
        .collect()
}

// /// Creates a dynamic buffer to store global data, and a descriptor set for those buffers to be used with offsets
// ///
// /// Returns vec of tuples containing the camera and scene subbuffers and the descriptor set with the offset for that frame
// pub fn create_double_global_descriptors<C: BufferContents, S: BufferContents>(
//     allocators: &Allocators,
//     device: &Arc<Device>,
//     descriptor_set_layout: Arc<DescriptorSetLayout>,
//     buffer_count: usize,
// ) -> Vec<(Subbuffer<C>, Subbuffer<S>, DescriptorSetWithOffsets)> {
//     let c_size = size_of::<C>() as DeviceSize;
//     let s_size = size_of::<S>() as DeviceSize;
//     let total_size = c_size + s_size;
//     let align = {
//         let min_dynamic_align = device
//             .physical_device()
//             .properties()
//             .min_uniform_buffer_offset_alignment
//             .as_devicesize();
//         // Round size up to the next multiple of align.
//         (total_size + min_dynamic_align - 1) & !(min_dynamic_align - 1)
//     };
//     let dynamic_buffer = {
//         let data_size = buffer_count as u64 * align;
//         Buffer::new_slice::<u8>(
//             allocators.memory.clone(),
//             BufferCreateInfo {
//                 usage: BufferUsage::UNIFORM_BUFFER,
//                 ..Default::default()
//             },
//             AllocationCreateInfo {
//                 memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
//                     | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
//                 ..Default::default()
//             },
//             data_size,
//         )
//         .unwrap()
//     };
//     // descriptor set is how we interface data between the buffer and the pipeline
//     let descriptor_set = PersistentDescriptorSet::new(
//         &allocators.descriptor_set,
//         descriptor_set_layout.clone(),
//         [
//             WriteDescriptorSet::buffer_with_range(
//                 0,
//                 DescriptorBufferInfo {
//                     buffer: dynamic_buffer.clone(),
//                     range: 0..c_size,
//                 },
//             ),
//             // WriteDescriptorSet::buffer(1, scenes_buffer.clone().index(i as DeviceSize)),
//             WriteDescriptorSet::buffer_with_range(
//                 1,
//                 DescriptorBufferInfo {
//                     buffer: dynamic_buffer.clone(),
//                     range: c_size..total_size,
//                 },
//             ),
//         ],
//         [],
//     )
//     .unwrap();
//     (0..buffer_count as DeviceSize)
//         .map(|i| {
//             let start = i * align;
//             let c_end = start + c_size;
//             let s_end = start + total_size;
//             let cam_buffer = dynamic_buffer
//                 .clone()
//                 .slice(start..c_end)
//                 .reinterpret::<C>();
//             let scene_buffer = dynamic_buffer
//                 .clone()
//                 .slice(c_end..s_end)
//                 .reinterpret::<S>();
//             let offset = (align * i) as u32;
//             let set = descriptor_set.clone().offsets([offset; 2]);
//             (cam_buffer, scene_buffer, set)
//         })
//         .collect()
// }

/// Create descriptor sets of a storage buffer containing an array of the given data type
pub fn create_storage_buffer<T: BufferContents>(
    allocators: &Allocators,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    object_count: usize,
) -> Uniform<[T]> {
    let storage_buffer = Buffer::new_slice(
        allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        object_count as DeviceSize,
    )
    .unwrap();

    let descriptor_set = PersistentDescriptorSet::new(
        &allocators.descriptor_set,
        descriptor_set_layout.clone(),
        [WriteDescriptorSet::buffer(0, storage_buffer.clone())],
        [],
    )
    .unwrap();

    (storage_buffer, descriptor_set)
}

/// Write to a storage buffer from an iterator
/// returns the index of the last item added or None if no items were added
pub fn write_to_storage_buffer<T: BufferContents>(
    buffer: &Subbuffer<[T]>,
    writes: impl Iterator<Item = impl Into<T>>,
    offset: usize,
) -> Option<usize> {
    let contents = buffer.write();
    // .unwrap_or_else(|e| panic!("Failed to write to storage buffer\n{}", e));
    let mut last = if offset == 0 { None } else { Some(offset) };

    match contents {
        Ok(mut guard) => {
            for (i, write) in writes.enumerate() {
                guard[i + offset] = write.into();
                last = Some(i + offset);
            }
        }
        Err(HostAccessError::AccessConflict(AccessConflict::DeviceRead)) => {
            println!("[Error] Storage buffer write blocked by GPU read")
        }
        Err(HostAccessError::AccessConflict(AccessConflict::DeviceWrite)) => {
            println!("[Error] Storage buffer write blocked by GPU write")
        }
        Err(HostAccessError::AccessConflict(AccessConflict::HostRead)) => {
            println!("[Error] Storage buffer write blocked by CPU read")
        }
        Err(HostAccessError::AccessConflict(AccessConflict::HostWrite)) => {
            println!("[Error] Storage buffer write blocked by CPU write")
        }
        Err(e) => println!("[Error] Failed to write to storage buffer, {}", e),
    }
    last
}

pub fn write_to_buffer<T: BufferContents>(buffer: &Subbuffer<T>, data: impl Into<T>) {
    let contents = buffer.write();
    // .unwrap_or_else(|e| panic!("Failed to write to buffer\n{}", e));
    match contents {
        Ok(mut guard) => *guard = data.into(),
        Err(HostAccessError::AccessConflict(AccessConflict::DeviceRead)) => {
            println!("[Error] Buffer write blocked by GPU read")
        }
        Err(HostAccessError::AccessConflict(AccessConflict::DeviceWrite)) => {
            println!("[Error] Buffer write blocked by GPU write")
        }
        Err(HostAccessError::AccessConflict(AccessConflict::HostRead)) => {
            println!("[Error] Buffer write blocked by CPU read")
        }
        Err(HostAccessError::AccessConflict(AccessConflict::HostWrite)) => {
            println!("[Error] Buffer write blocked by CPU write")
        }
        Err(e) => println!("[Error] Failed to write to buffer, {}", e),
    }
}
