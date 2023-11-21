use std::{fs::File, path::Path, sync::Arc};

use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
        PrimaryCommandBufferAbstract,
    },
    device::Queue,
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::GpuFuture,
    DeviceSize,
};

use crate::vulkano_objects::allocators::Allocators;

/// load a png texture into a ViewImage
pub fn load_texture(allocators: &Allocators, queue: &Arc<Queue>, path: &Path) -> Arc<ImageView> {
    // decode png
    let decoder = png::Decoder::new(File::open(path).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info();
    let extent = [info.width, info.height, 1];

    println!("Texture gamme: {:?}", info.source_gamma);

    // create image
    let image = Image::new(
        allocators.memory.clone(),
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::R8G8B8A8_SRGB,
            extent,
            usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )
    .unwrap();

    // create staging buffer
    let staging_buffer = Buffer::new_slice(
        allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        (info.width * info.height * 4) as DeviceSize,
    )
    .unwrap();

    // write to staging buffer
    reader
        .next_frame(&mut staging_buffer.write().unwrap())
        .unwrap();

    // copy to image
    let mut builder = AutoCommandBufferBuilder::primary(
        &allocators.command_buffer,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    builder
        .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            staging_buffer,
            image.clone(),
        ))
        .unwrap();

    // send it
    builder
        .build()
        .unwrap()
        .execute(queue.clone())
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    ImageView::new_default(image).unwrap()
}
