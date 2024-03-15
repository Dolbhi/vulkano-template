use std::sync::Arc;
use std::vec;

use vulkano::{sync::GpuFuture, Validated, VulkanError};

use winit::event_loop::EventLoop;

use crate::FRAME_PROFILER;

use super::renderer::Renderer;
use super::{context::Context, context::Fence};

pub struct RenderLoop {
    pub context: Context,
    recreate_swapchain: bool,
    window_resized: bool,
    fences: Vec<Option<Arc<Fence>>>,
    previous_frame_i: u32,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let context: Context = Context::initialize(event_loop);
        let fences = vec![None; context.get_image_count()];

        Self {
            context,
            recreate_swapchain: false,
            window_resized: false,
            fences,
            previous_frame_i: 0,
        }
    }

    /// update renderer and draw upcoming image
    ///
    /// `upload_render_data` will be called once the swapchain image is ready
    pub fn update<R, F>(&mut self, renderer: &mut R, upload_render_data: F)
    where
        R: Renderer,
        F: FnOnce(&mut R, usize),
    {
        let now = std::time::Instant::now();

        // check zero sized window
        let image_extent: [u32; 2] = self.context.window.inner_size().into();
        if image_extent.contains(&0) {
            return;
        }

        // do recreation if necessary
        if self.window_resized {
            self.window_resized = false;
            self.context.handle_window_resize();
            renderer.recreate_framebuffers(&self.context);
            renderer.recreate_pipelines(&self.context);
        } else if self.recreate_swapchain {
            self.recreate_swapchain = false;
            self.context.recreate_swapchain();
            renderer.recreate_framebuffers(&self.context);
        }

        // get upcoming image to display and future of when it is ready
        let (image_i, suboptimal, acquire_future) = match self.context.acquire_swapchain_image() {
            Ok(r) => r,
            Err(Validated::Error(VulkanError::OutOfDate)) => {
                self.recreate_swapchain = true;
                return;
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };
        if suboptimal {
            self.recreate_swapchain = true;
        }

        // println!("Pre-render      {:>4} μs", now.elapsed().as_micros());
        // let mut profiler = unsafe { FRAME_PROFILER.take().unwrap() };
        // profiler.add_sample(now.elapsed().as_micros() as u32, 1);
        let pre_ren_time = now.elapsed().as_micros() as u32;
        let now = std::time::Instant::now();

        // wait for upcoming image to be ready (it should be by this point)
        let index = image_i as usize;
        if let Some(image_fence) = &mut self.fences[index] {
            image_fence.wait(None).unwrap();
            image_fence.cleanup_finished();
        }

        // println!("Frame cleanup   {:>4} μs", now.elapsed().as_micros());
        // profiler.add_sample(now.elapsed().as_micros() as u32, 2);
        let frame_clean_time = now.elapsed().as_micros() as u32;
        let now = std::time::Instant::now();

        // let renderer = renderer.upload_data(index);
        upload_render_data(renderer, index);

        // println!("Render upload   {:>4} μs", now.elapsed().as_micros());
        // let mut profiler = unsafe { FRAME_PROFILER.take().unwrap() };
        // profiler.add_sample(now.elapsed().as_micros() as u32, 3);
        let ren_up_time = now.elapsed().as_micros() as u32;
        let now = std::time::Instant::now();

        // logic that uses the GPU resources that are currently not used (have been waited upon)
        let something_needs_all_gpu_resources = false;
        let previous_future = match self.fences[self.previous_frame_i as usize].clone() {
            None => self.context.synchronize().boxed(),
            Some(fence) => {
                if something_needs_all_gpu_resources {
                    fence.wait(None).unwrap();
                }
                fence.boxed()
            }
        };
        if something_needs_all_gpu_resources {
            // logic that can use every GPU resource (the GPU is sleeping)
        }

        // println!("Last frame wait {:>4} μs", now.elapsed().as_micros());
        // let now = std::time::Instant::now();
        // profiler.add_sample(now.elapsed().as_micros() as u32, 4);
        let last_f_wait = now.elapsed().as_micros() as u32;
        unsafe {
            let mut profiler = FRAME_PROFILER.take().unwrap();

            profiler.add_sample(pre_ren_time, 1);
            profiler.add_sample(frame_clean_time, 2);
            profiler.add_sample(ren_up_time, 3);
            profiler.add_sample(last_f_wait, 4);

            FRAME_PROFILER = Some(profiler);
        }

        // RENDER
        // println!("[Pre-render state] seconds_passed: {}, image_i: {}, window_resized: {}, recreate_swapchain: {}", seconds_passed, image_i, self.window_resized, self.recreate_swapchain);
        let result =
            self.context
                .flush_next_future(previous_future, acquire_future, image_i, |b| {
                    renderer.build_command_buffer(index, b)
                });
        // replace fence of upcoming image with new one
        self.fences[image_i as usize] = match result {
            Ok(fence) => Some(Arc::new(fence)),
            Err(Validated::Error(VulkanError::OutOfDate)) => {
                self.recreate_swapchain = true;
                None
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                None
            }
        };

        // println!("\rFlush future    {:>4} μs", now.elapsed().as_micros());

        self.previous_frame_i = image_i;
    }

    pub fn handle_window_resize(&mut self) {
        // impacts the next update
        self.window_resized = true;
    }
}
