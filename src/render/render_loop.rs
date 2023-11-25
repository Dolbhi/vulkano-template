use std::iter::zip;
// use std::mem::size_of;
use std::path::Path;
use std::sync::Arc;

use vulkano::descriptor_set::{DescriptorSet, PersistentDescriptorSet};
use vulkano::DeviceSize;
use vulkano::{sync::GpuFuture, Validated, VulkanError};

use winit::event_loop::EventLoop;

use super::{
    render_data::{frame_data::FrameData, mesh::Mesh, render_object::RenderObject},
    renderer::Renderer,
};
use crate::render::render_data::texture::{create_sampler, load_texture};
use crate::vulkano_objects::buffers::Buffers;
use crate::VertexFull;
use crate::{
    game_objects::Camera,
    shaders::{basic, uv},
};

pub struct RenderLoop {
    renderer: Renderer,
    recreate_swapchain: bool,
    window_resized: bool,
    frames: Vec<FrameData>,
    previous_frame_i: u32,
    global_descriptor: Arc<PersistentDescriptorSet>,
    global_alignment: DeviceSize,
    total_seconds: f32,
    render_objects: Vec<RenderObject>,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let mut renderer = Renderer::initialize(event_loop);

        // pipelines
        let basic_shader_id = String::from("basic");
        // let alpha_shader_id = String::from("alpha");
        let uv_shader_id = String::from("uv");
        {
            let vertex_shader = basic::vs::load(renderer.clone_device())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            let fragment_shader = basic::fs::load(renderer.clone_device())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            renderer.init_pipeline(basic_shader_id.clone(), vertex_shader, fragment_shader);

            // let vertex_shader = alpha::vs::load(renderer.clone_device())
            //     .expect("failed to create shader module")
            //     .entry_point("main")
            //     .unwrap();
            // let fragment_shader = alpha::fs::load(renderer.clone_device())
            //     .expect("failed to create shader module")
            //     .entry_point("main")
            //     .unwrap();
            // renderer.init_pipeline(alpha_shader_id.clone(), vertex_shader, fragment_shader);

            let vertex_shader = uv::vs::load(renderer.clone_device())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            let fragment_shader = uv::fs::load(renderer.clone_device())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            renderer.init_pipeline(uv_shader_id.clone(), vertex_shader, fragment_shader);
        }

        // Texture
        let le_texture = load_texture(
            &renderer.allocators,
            &renderer.queue,
            Path::new("models/lost_empire-RGBA.png"),
        );

        let ina_textures = [
            "models/ina/Hair_Base_Color.png",
            "models/ina/Cloth_Base_Color.png",
            "models/ina/Body_Base_Color.png",
            "models/ina/Head_Base_Color.png",
        ]
        .map(|p| load_texture(&renderer.allocators, &renderer.queue, Path::new(p)));

        let linear_sampler = create_sampler(
            renderer.device.clone(),
            vulkano::image::sampler::Filter::Linear,
        );

        // materials
        //  lost empire
        let le_mat = renderer.init_material_with_texture(
            basic_shader_id.clone(),
            le_texture,
            linear_sampler.clone(),
        );

        //  ina
        let ina_materials = ina_textures.map(|tex| {
            renderer.init_material_with_texture(
                basic_shader_id.clone(),
                tex,
                linear_sampler.clone(),
            )
        });

        //  uv
        let uv_mat = renderer.init_material(uv_shader_id.clone());

        // meshes
        //      gun
        // let Mesh(vertices, indices) = Mesh::from_obj("models/gun.obj").pop().unwrap();
        // let gun_mesh = renderer.init_mesh(vertices, indices);

        //      suzanne
        let Mesh(vertices, indices) = Mesh::from_obj(Path::new("models/suzanne.obj"))
            .pop()
            .unwrap();
        let suzanne = renderer.init_mesh(vertices, indices);

        //      square
        let vertices = vec![
            VertexFull {
                position: [-0.25, -0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
            },
            VertexFull {
                position: [0.25, -0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
            },
            VertexFull {
                position: [-0.25, 0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
            },
            VertexFull {
                position: [0.25, 0.25, 0.0],
                normal: [0.0, 0.0, 1.0],
                colour: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
            },
        ];
        let indices = vec![0, 1, 2, 2, 1, 3];
        let square = renderer.init_mesh(vertices, indices);

        //      lost empire
        let le_meshes: Vec<Arc<Buffers<VertexFull>>> =
            Mesh::from_obj(Path::new("models/lost_empire.obj"))
                .into_iter()
                .map(|Mesh(vertices, indices)| renderer.init_mesh(vertices, indices))
                .collect();
        println!("Lost empire mesh count: {}", le_meshes.len());

        //      ina
        let ina_meshes: Vec<Arc<Buffers<VertexFull>>> =
            Mesh::from_obj(Path::new("models/ina/ReadyToRigINA.obj"))
                .into_iter()
                .skip(2)
                .map(|Mesh(vertices, indices)| renderer.init_mesh(vertices, indices))
                .collect();
        println!("Ina mesh count: {}", ina_meshes.len());

        renderer.debug_assets();

        // objects
        let mut render_objects = Vec::<RenderObject>::new();
        //  Suzanne
        render_objects.push(RenderObject::new(suzanne, uv_mat.clone()));

        //  Squares
        for (x, y, z) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
            let mut square_obj = RenderObject::new(square.clone(), uv_mat.clone());
            square_obj.update_transform([x as f32, y as f32, z as f32], cgmath::Rad(0.));
            render_objects.push(square_obj)
        }

        //  Ina
        for (mesh, mat) in zip(ina_meshes, ina_materials) {
            let mut obj = RenderObject::new(mesh, mat);
            obj.update_transform([0.0, 5.0, -1.0], cgmath::Rad(0.));
            render_objects.push(obj);
        }

        //  lost empires
        for mesh in le_meshes {
            let le_obj = RenderObject::new(mesh, le_mat.clone());
            render_objects.push(le_obj);
        }

        // global descriptors TODO: 1. Group dyanamics into its own struct 2. create independent layout not based on mat
        let (global_alignment, global_buffers, global_descriptor) =
            renderer.create_scene_buffers(&String::from("basic"));

        let object_uniforms = renderer.create_object_buffers(&String::from("basic"));

        // create frame data
        let frames = zip(global_buffers, object_uniforms)
            .into_iter()
            .map(
                |((cam_buffer, scene_buffer), (storage_buffer, object_descriptor))| {
                    FrameData::new(cam_buffer, scene_buffer, storage_buffer, object_descriptor)
                },
            )
            .collect();

        Self {
            renderer,
            recreate_swapchain: false,
            window_resized: false,
            frames,
            previous_frame_i: 0,
            global_descriptor,
            global_alignment,
            total_seconds: 0.0,
            render_objects,
        }
    }

    /// write gpu data to respective buffers
    fn update_gpu_data(&mut self, camera_data: &Camera, image_i: u32) {
        let frame = &mut self.frames[image_i as usize];

        // update object data
        self.render_objects[0].update_transform([0., 0., 0.], cgmath::Rad(self.total_seconds));
        frame.update_objects_data(&self.render_objects);

        // update camera
        let extends = self.renderer.window.inner_size();
        frame.update_camera_data(
            camera_data.view_matrix(),
            camera_data.projection_matrix(extends.width as f32 / extends.height as f32),
        );

        // update scene data
        frame.update_scene_data([self.total_seconds.sin(), 0., self.total_seconds.cos(), 1.]);
    }

    /// update renderer and draw upcoming image
    pub fn update(&mut self, transform_data: &Camera, seconds_passed: f32) {
        // stuff
        self.total_seconds += seconds_passed;

        // check zero sized window
        let image_extent: [u32; 2] = self.renderer.window.inner_size().into();
        if image_extent.contains(&0) {
            return;
        }

        // do recreation if necessary
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain = false;
            self.renderer.handle_window_resize();
        } else if self.recreate_swapchain {
            self.recreate_swapchain = false;
            self.renderer.recreate_swapchain();
            // self.renderer.recreate_cb();
        }

        // get upcoming image to display and future of when it is ready
        let (image_i, suboptimal, acquire_future) = match self.renderer.acquire_swapchain_image() {
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

        // wait for upcoming image to be ready (it should be by this point)
        if let Some(image_fence) = &mut self.frames[image_i as usize].fence {
            // image_fence.wait(None).unwrap();
            image_fence.cleanup_finished();
        }

        self.update_gpu_data(transform_data, image_i);

        // logic that uses the GPU resources that are currently not used (have been waited upon)
        let something_needs_all_gpu_resources = false;
        let previous_future = match self.frames[self.previous_frame_i as usize].fence.clone() {
            None => self.renderer.synchronize().boxed(),
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

        // RENDER
        // println!("[Pre-render state] seconds_passed: {}, image_i: {}, window_resized: {}, recreate_swapchain: {}", seconds_passed, image_i, self.window_resized, self.recreate_swapchain);
        // println!(
        //     "[Camera dynamics] data size: {}, alignment: {}, buffer size: {}, range end: {}, offset: {}",
        //     size_of::<GPUCameraData>(),
        //     self.camera_dynamic.align(),
        //     self.camera_dynamic.clone_buffer().size(),
        //     (0..size_of::<GPUCameraData>()).end,
        //     self.camera_dynamic.align() as u32 * image_i,
        // );
        let result = self.renderer.flush_next_future(
            previous_future,
            acquire_future,
            image_i,
            &self.render_objects,
            self.global_descriptor.clone().offsets([
                self.global_alignment as u32 * image_i,
                self.global_alignment as u32 * image_i,
            ]),
            self.frames[image_i as usize]
                .get_objects_descriptor()
                .clone(),
        );
        // replace fence of upcoming image with new one
        self.frames[image_i as usize].fence = match result {
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
        self.previous_frame_i = image_i;
    }

    pub fn handle_window_resize(&mut self) {
        // impacts the next update
        self.window_resized = true;
    }

    pub fn handle_window_wait(&self) {
        self.renderer.window.request_redraw();
    }
}
