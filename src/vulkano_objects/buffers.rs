use std::{marker::PhantomData, mem::size_of, sync::Arc};

use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecFuture, CommandBufferUsage, CopyBufferInfo,
        PrimaryCommandBufferAbstract,
    },
    descriptor_set::{
        layout::DescriptorSetLayout, DescriptorBufferInfo, PersistentDescriptorSet,
        WriteDescriptorSet,
    },
    device::{Device, Queue},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::graphics::vertex_input::Vertex,
    sync::{future::NowFuture, GpuFuture},
    DeviceSize,
};

use super::allocators::Allocators;

pub type Uniform<U> = (Subbuffer<U>, Arc<PersistentDescriptorSet>);

/// Struct with a vertex and index, using VertexFull for vertices
pub struct Buffers<V: Vertex + BufferContents> {
    pub vertex: Subbuffer<[V]>,
    pub index: Subbuffer<[u32]>,
    // pub uniforms: Vec<Uniform<U>>,
}

impl<V: Vertex + BufferContents> Buffers<V> {
    /// Creates simple vertex, index and uniform buffers of specified model
    // pub fn initialize_host_accessible<M: Model<V, U>>(
    //     allocators: &Allocators,
    //     descriptor_set_layout: Arc<DescriptorSetLayout>,
    //     uniform_buffer_count: usize,
    // ) -> Self {
    //     Self {
    //         vertex: create_cpu_accessible_vertex::<V, U, M>(allocators),
    //         index: create_cpu_accessible_index::<V, U, M>(allocators),
    //         uniforms: create_cpu_accessible_uniforms::<V, U, M>(
    //             allocators,
    //             descriptor_set_layout,
    //             uniform_buffer_count,
    //         ),
    //     }
    // }

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
}

/// returns simple cpu accessible vertex buffer
// fn create_cpu_accessible_vertex<V, U, M>(allocators: &Allocators) -> Subbuffer<[V]>
// where
//     V: BufferContents,
//     U: BufferContents,
//     M: Model<V, U>,
// {
//     Buffer::from_iter(
//         &allocators.memory,
//         BufferCreateInfo {
//             usage: BufferUsage::VERTEX_BUFFER,
//             ..Default::default()
//         },
//         AllocationCreateInfo {
//             usage: MemoryUsage::Upload,
//             ..Default::default()
//         },
//         M::get_vertices(),
//     )
//     .unwrap()
// }

/// returns simple cpu accessible index buffer
// fn create_cpu_accessible_index<V, U, M>(allocators: &Allocators) -> Subbuffer<[u16]>
// where
//     V: BufferContents,
//     U: BufferContents,
//     M: Model<V, U>,
// {
//     Buffer::from_iter(
//         &allocators.memory,
//         BufferCreateInfo {
//             usage: BufferUsage::INDEX_BUFFER,
//             ..Default::default()
//         },
//         AllocationCreateInfo {
//             usage: MemoryUsage::Upload,
//             ..Default::default()
//         },
//         M::get_indices(),
//     )
//     .unwrap()
// }

/// returns a device only buffer and a future that copies data to it
fn create_device_local_buffer<T: BufferContents>(
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

/// returns uniform buffers with corresponding descriptor sets for interfacing
pub fn create_cpu_accessible_uniforms<U: BufferContents + Clone>(
    allocators: &Allocators,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    buffer_count: usize,
    initial_uniform: U,
) -> Vec<Uniform<U>> {
    (0..buffer_count)
        .map(|_| {
            let buffer = Buffer::from_data(
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
                initial_uniform.clone(),
            )
            .unwrap();

            // descriptor set is how we interface data between the buffer and the pipeline
            let descriptor_set = PersistentDescriptorSet::new(
                &allocators.descriptor_set,
                descriptor_set_layout.clone(),
                [WriteDescriptorSet::buffer(0, buffer.clone())],
                [],
            )
            .unwrap();

            (buffer, descriptor_set)
        })
        .collect()
}

pub struct DynamicBuffer<T: BufferContents> {
    buffer: Subbuffer<[u8]>,
    // count: usize,
    align: DeviceSize,
    marker: PhantomData<T>,
}

impl<T: BufferContents> DynamicBuffer<T> {
    fn new(allocators: &Allocators, count: DeviceSize, device: &Arc<Device>) -> Self {
        let align = Self::calculate_align(device);
        let data_size = count * align;

        let buffer = Buffer::new_slice::<u8>(
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
        .unwrap();

        DynamicBuffer {
            buffer,
            align,
            marker: PhantomData,
        }
    }

    const fn elem_size() -> DeviceSize {
        size_of::<T>() as DeviceSize
    }

    fn calculate_align(device: &Arc<Device>) -> DeviceSize {
        let min_dynamic_align = device
            .physical_device()
            .properties()
            .min_uniform_buffer_offset_alignment
            .as_devicesize();

        // Round size up to the next multiple of align.
        (Self::elem_size() + min_dynamic_align - 1) & !(min_dynamic_align - 1)
    }

    pub fn clone_buffer(&self) -> Subbuffer<[u8]> {
        self.buffer.clone()
    }
    pub fn align(&self) -> DeviceSize {
        self.align
    }

    pub fn reinterpret(&self, index: usize) -> Subbuffer<T> {
        let start = self.align * index as DeviceSize;
        let end = start + size_of::<T>() as DeviceSize;
        self.buffer.clone().slice(start..end).reinterpret()
    }
}

pub fn create_global_descriptors<C: BufferContents, S: BufferContents>(
    allocators: &Allocators,
    device: &Arc<Device>,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    buffer_count: usize,
) -> (
    DynamicBuffer<C>,
    DynamicBuffer<S>,
    Arc<PersistentDescriptorSet>,
) {
    let camera_dynamic = DynamicBuffer::<C>::new(allocators, buffer_count as DeviceSize, device);
    // println!(
    //     "[Camera dynamics] data size: {}, alignment: {}, buffer size: {}",
    //     size_of::<C>(),
    //     camera_dynamic.align,
    //     camera_dynamic.buffer.size(),
    // );
    let scene_dynamic = DynamicBuffer::<S>::new(allocators, buffer_count as DeviceSize, device);
    // println!(
    //     "[Scene dynamics] data size: {}, alignment: {}, buffer size: {}",
    //     size_of::<S>(),
    //     scene_dynamic.align,
    //     scene_dynamic.buffer.size(),
    // );

    // descriptor set is how we interface data between the buffer and the pipeline
    let descriptor_set = PersistentDescriptorSet::new(
        &allocators.descriptor_set,
        descriptor_set_layout.clone(),
        [
            WriteDescriptorSet::buffer_with_range(
                0,
                DescriptorBufferInfo {
                    buffer: camera_dynamic.buffer.clone(),
                    range: 0..size_of::<C>() as DeviceSize,
                },
            ),
            // WriteDescriptorSet::buffer(1, scenes_buffer.clone().index(i as DeviceSize)),
            WriteDescriptorSet::buffer_with_range(
                1,
                DescriptorBufferInfo {
                    buffer: scene_dynamic.buffer.clone(),
                    range: 0..size_of::<S>() as DeviceSize,
                },
            ),
        ],
        [],
    )
    .unwrap();

    (camera_dynamic, scene_dynamic, descriptor_set)
}

pub fn create_storage_buffers<T: BufferContents>(
    allocators: &Allocators,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    buffer_count: usize,
    object_count: usize,
) -> Vec<Uniform<[T]>> {
    (0..buffer_count)
        .map(|_| {
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
        })
        .collect()
}
