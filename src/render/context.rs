use std::sync::Arc;

use crate::{
    vulkano_objects::{self, allocators::Allocators},
    RENDER_PROFILER,
};
use egui_winit_vulkano::{Gui, GuiConfig};
use vulkano::{
    command_buffer::{self, AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    device::{Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo},
    image::{
        view::{ImageView, ImageViewCreateInfo},
        Image,
    },
    instance::Instance,
    pipeline::graphics::viewport::Viewport,
    swapchain::{
        self, PresentFuture, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
        SwapchainPresentInfo,
    },
    sync::{
        self,
        future::{FenceSignalFuture, NowFuture},
        GpuFuture,
    },
    Validated, VulkanError,
};
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub type Fence = FenceSignalFuture<PresentFuture<Box<dyn GpuFuture>>>;

const INIT_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(1000.0f32, 600.0);

/// All relevant structs for rendering to a single window including GUI
///
/// Has implementations for swapchain recreation and command buffer building and executing
pub struct Context {
    _instance: Arc<Instance>,
    /// For get inner size and request redraw
    pub window: Arc<Window>,
    // pub surface: Arc<Surface>, // for making gui
    /// Just for pipeline creation
    pub viewport: Viewport,
    pub device: Arc<Device>,
    /// For submitting command buffers
    pub queue: Arc<Queue>,
    pub allocators: Allocators,
    /// For swapchain recreation and image presenting
    pub swapchain: Arc<Swapchain>,
    pub images: Vec<Arc<Image>>,
    // pub gui_image_views: Vec<Arc<ImageView>>,
    pub gui: Gui,
}

impl Context {
    pub fn initialize(event_loop: &EventLoop<()>) -> Self {
        let instance = vulkano_objects::instance::get_instance(event_loop);

        let window = Arc::new(WindowBuilder::new().build(event_loop).unwrap());
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        // window settings
        window.set_title("Rusty Renderer");
        window.set_inner_size(INIT_WINDOW_SIZE);

        let viewport: Viewport = Viewport {
            extent: window.inner_size().into(),
            ..Default::default() // offset: [0.0, 0.0],
                                 // depth_range: 0.0..=1.0,
        };

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_shader_draw_parameters: true,
            khr_image_format_list: true,
            khr_swapchain_mutable_format: true,
            ..DeviceExtensions::empty()
        };
        let device_features = Features {
            fill_mode_non_solid: true,
            ..Features::empty()
        };
        let (physical_device, queue_family_index) =
            vulkano_objects::physical_device::select_physical_device(
                &instance,
                surface.clone(),
                &device_extensions,
                &device_features,
            );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions, // new
                enabled_features: device_features,
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let allocators = Allocators::new(device.clone());

        let queue = queues.next().unwrap();

        let (swapchain, images) = vulkano_objects::swapchain::create_swapchain(
            &physical_device,
            device.clone(),
            surface.clone(),
        );
        // let gui_image_views = images
        //     .iter()
        //     .map(|image| {
        //         ImageView::new(
        //             image.clone(),
        //             ImageViewCreateInfo {
        //                 format: vulkano::format::Format::B8G8R8A8_UNORM,
        //                 ..ImageViewCreateInfo::from_image(&image)
        //             },
        //         )
        //         .unwrap()
        //     })
        //     .collect();
        let gui = Gui::new(
            event_loop,
            surface.clone(),
            queue.clone(),
            vulkano::format::Format::B8G8R8A8_UNORM,
            GuiConfig {
                is_overlay: true,
                ..Default::default()
            },
        );

        println!(
            "[Render Context Info]\nswapchain image count: {}\nQueue family: {}\nSwapchain format: {:?}",
            images.len(),
            queue_family_index,
            swapchain.image_format(),
        );

        // // auto focus window
        // window.focus_window();

        Self {
            _instance: instance,
            window,
            viewport,
            device,
            queue,
            allocators,
            swapchain,
            images,
            // gui_image_views,
            gui,
        }
    }

    pub fn get_image_count(&self) -> usize {
        self.images.len()
        // self.swapchain.image_count() as usize
    }

    /// recreates swapchain and swapchain images
    pub fn recreate_swapchain(&mut self) {
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: self.window.inner_size().into(),
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            // Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = new_swapchain;
        self.images = new_images;
        // self.gui_image_views = self
        //     .images
        //     .iter()
        //     .map(|image| {
        //         ImageView::new(
        //             image.clone(),
        //             ImageViewCreateInfo {
        //                 format: vulkano::format::Format::B8G8R8A8_UNORM,
        //                 ..ImageViewCreateInfo::from_image(&image)
        //             },
        //         )
        //         .unwrap()
        //     })
        //     .collect();
    }
    pub fn handle_window_resize(&mut self) {
        self.recreate_swapchain();
        self.viewport.extent = self.window.inner_size().into();
    }

    /// Gets future where next image in swapchain is ready
    pub fn acquire_swapchain_image(
        &self,
    ) -> Result<(u32, bool, SwapchainAcquireFuture), Validated<VulkanError>> {
        swapchain::acquire_next_image(self.swapchain.clone(), None)
    }

    pub fn synchronize(&self) -> NowFuture {
        let mut now = sync::now(self.device.clone());
        now.cleanup_finished();

        now
    }

    /// Join given futures then execute new commands and present the swapchain image corresponding to the given image_i
    pub fn flush_next_future<F>(
        &mut self,
        previous_future: Box<dyn GpuFuture>,
        swapchain_acquire_future: SwapchainAcquireFuture,
        image_i: u32,
        build_commands: F,
    ) -> Result<Fence, Validated<VulkanError>>
    where
        F: FnOnce(&mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>),
    {
        let now = std::time::Instant::now();

        // create builder
        let mut builder = command_buffer::AutoCommandBufferBuilder::primary(
            &self.allocators.command_buffer,
            self.queue.queue_family_index(),
            command_buffer::CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        build_commands(&mut builder);

        // println!("\rComBuf building {:>4} μs", now.elapsed().as_micros());
        // let mut profiler = unsafe { FRAME_PROFILER.take().unwrap() };

        // profiler.add_sample(now.elapsed().as_micros() as u32, 5);
        let combuf_time = now.elapsed().as_micros() as u32;
        let now = std::time::Instant::now();

        // Join given futures then execute new draw commands
        let draw_future = previous_future
            .join(swapchain_acquire_future)
            .then_execute(self.queue.clone(), builder.build().unwrap())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        let image = &self.images[image_i as usize];
        // cache this????
        let gui_image_view = ImageView::new(
            image.clone(),
            ImageViewCreateInfo {
                format: vulkano::format::Format::B8G8R8A8_UNORM,
                ..ImageViewCreateInfo::from_image(image)
            },
        )
        .unwrap();

        // Execute new GUI commands, then present the swapchain image corresponding to the given image_i
        let result = self
            .gui
            .draw_on_image(draw_future, gui_image_view)
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();

        // println!(
        //     "\rExecute ComBuf  {:>4} μs       ",
        //     now.elapsed().as_micros()
        // );
        // profiler.add_sample(now.elapsed().as_micros() as u32, 6);
        let exe_time = now.elapsed().as_micros() as u32;
        unsafe {
            let mut profiler = RENDER_PROFILER.take().unwrap();

            profiler.add_sample(combuf_time, 5);
            profiler.add_sample(exe_time, 6);

            RENDER_PROFILER = Some(profiler);
        }

        result
    }
}
