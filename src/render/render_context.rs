use std::sync::Arc;

use vulkano::{
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    instance::Instance,
    swapchain::Surface,
};
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{CursorGrabMode, Window, WindowBuilder},
};

use crate::vulkano_objects::{self, allocators::Allocators};

const INIT_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(1000.0f32, 600.0);

pub struct RenderContext {
    _instance: Arc<Instance>,
    pub window: Arc<Window>, // pending refactor with swapchain
    pub device: Arc<Device>,
    pub queue: Arc<Queue>, // for submitting command buffers
    pub allocators: Allocators,
}

impl RenderContext {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let instance = vulkano_objects::instance::get_instance(event_loop);

        let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        // window settings
        window.set_title("Rusty Renderer");
        let _new_size = window.request_inner_size(INIT_WINDOW_SIZE);
        window.set_cursor_visible(false);
        window
            .set_cursor_grab(CursorGrabMode::Confined)
            .or_else(|_e| window.set_cursor_grab(CursorGrabMode::Locked))
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_shader_draw_parameters: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) =
            vulkano_objects::physical_device::select_physical_device(
                &instance,
                surface.clone(),
                &device_extensions,
            );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions, // new
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let allocators = Allocators::new(device.clone());

        let queue = queues.next().unwrap();

        Self {
            _instance: instance,
            window,
            device,
            queue,
            allocators,
        }
    }
}
