use std::{fs::File, path::Path, sync::Arc};

use png::ColorType;
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

// pub enum PNGFormat {
//     Transparent,
//     NonTransparent,
// }

/// load a png texture into a ViewImage
pub fn load_texture(allocators: &Allocators, queue: &Arc<Queue>, path: &Path) -> Arc<ImageView> {
    // decode png
    let decoder = png::Decoder::new(File::open(path).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let info = reader.info();
    let extent = [info.width, info.height, 1];

    // println!("Texture gamme: {:?}", info.source_gamma);

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
    match info.color_type {
        ColorType::Rgb => {
            let pixel_count = (info.width * info.height) as usize;

            let mut rgb_buf = vec![0; pixel_count * 3];
            reader.next_frame(rgb_buf.as_mut_slice()).unwrap();

            let mut rgba_buf = staging_buffer.write().unwrap();
            for i in 0..pixel_count {
                rgba_buf[i * 4] = rgb_buf[i * 3];
                rgba_buf[i * 4 + 1] = rgb_buf[i * 3 + 1];
                rgba_buf[i * 4 + 2] = rgb_buf[i * 3 + 2];
                rgba_buf[i * 4 + 3] = u8::MAX;
            }
        }
        ColorType::Rgba => {
            reader
                .next_frame(&mut staging_buffer.write().unwrap())
                .unwrap();
        }
        _ => {
            panic!(
                "Trying to load texture with unsupported color type: {:?}",
                info.color_type
            )
        }
    }

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
