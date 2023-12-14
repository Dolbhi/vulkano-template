use std::sync::Arc;
use std::vec;

use cgmath::Matrix4;
use vulkano::{sync::GpuFuture, Validated, VulkanError};

use winit::event_loop::EventLoop;

use super::renderer::Fence;
use super::{
    render_data::{render_object::RenderObject, DrawSystem},
    renderer::Renderer,
};

use crate::{
    game_objects::Camera,
    shaders::{
        basic::{self, vs::GPUObjectData},
        phong, uv,
    },
};

pub struct RenderLoop {
    pub renderer: Renderer,
    pub render_data: DrawSystem<GPUObjectData, Matrix4<f32>>,
    recreate_swapchain: bool,
    window_resized: bool,
    fences: Vec<Option<Arc<Fence>>>,
    previous_frame_i: u32,
}

impl RenderLoop {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let renderer = Renderer::initialize(event_loop);

        let render_data = Self::init_render_objects(&renderer);

        let fences = vec![None; renderer.swapchain.image_count() as usize]; //(0..frames.len()).map(|_| None).collect();

        Self {
            renderer,
            render_data,
            recreate_swapchain: false,
            window_resized: false,
            fences,
            previous_frame_i: 0,
        }
    }

    /// update renderer and draw upcoming image
    pub fn update<'a>(
        &mut self,
        camera_data: &Camera,
        render_objects: impl Iterator<Item = &'a Arc<RenderObject<Matrix4<f32>>>>,
    ) {
        // check zero sized window
        let image_extent: [u32; 2] = self.renderer.window.inner_size().into();
        if image_extent.contains(&0) {
            return;
        }

        // do recreation if necessary
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain = false;
            self.renderer.recreate_swapchain();
            self.render_data.recreate_pipelines(&self.renderer);
        } else if self.recreate_swapchain {
            self.recreate_swapchain = false;
            self.renderer.recreate_swapchain();
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
        if let Some(image_fence) = &mut self.fences[image_i as usize] {
            // image_fence.wait(None).unwrap();
            image_fence.cleanup_finished();
        }

        let extends = self.renderer.window.inner_size();
        self.render_data.upload_draw_data(
            render_objects,
            camera_data,
            extends.width as f32 / extends.height as f32,
            image_i,
        );

        // logic that uses the GPU resources that are currently not used (have been waited upon)
        let something_needs_all_gpu_resources = false;
        let previous_future = match self.fences[self.previous_frame_i as usize].clone() {
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
        let result = self.renderer.flush_next_future(
            previous_future,
            acquire_future,
            image_i,
            &mut self.render_data,
        );
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
        self.previous_frame_i = image_i;
    }

    pub fn handle_window_resize(&mut self) {
        // impacts the next update
        self.window_resized = true;
    }
    pub fn handle_window_wait(&self) {
        self.renderer.window.request_redraw();
    }

    fn init_render_objects(renderer: &Renderer) -> DrawSystem<GPUObjectData, Matrix4<f32>> {
        // pipelines
        let mut data = {
            let vertex_shader = basic::vs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            let fragment_shader = basic::fs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            DrawSystem::new(&renderer, vertex_shader, fragment_shader)
        };
        let phong_id = {
            let vertex_shader = phong::vs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            let fragment_shader = phong::fs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            data.add_pipeline(&renderer, vertex_shader, fragment_shader)
        };
        let uv_id = {
            let vertex_shader = uv::vs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            let fragment_shader = uv::fs::load(renderer.device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .unwrap();
            data.add_pipeline(&renderer, vertex_shader, fragment_shader)
        };

        // // Texture
        // let le_texture = load_texture(
        //     &renderer.allocators,
        //     &renderer.queue,
        //     Path::new("models/lost_empire-RGBA.png"),
        // );

        // let ina_textures = [
        //     "models/ina/Hair_Base_Color.png",
        //     "models/ina/Cloth_Base_Color.png",
        //     "models/ina/Body_Base_Color.png",
        //     "models/ina/Head_Base_Color.png",
        // ]
        // .map(|p| load_texture(&renderer.allocators, &renderer.queue, Path::new(p)));

        // let linear_sampler = create_sampler(
        //     renderer.device.clone(),
        //     vulkano::image::sampler::Filter::Linear,
        // );

        // // materials
        // //  lost empire
        // let le_mat_id = data.add_material(
        //     phong_id,
        //     "lost_empire",
        //     Some(data.get_pipeline(phong_id).create_material_set(
        //         &renderer.allocators,
        //         2,
        //         le_texture,
        //         linear_sampler.clone(),
        //     )),
        // );

        // //  ina
        // let ina_ids: Vec<crate::render::render_data::material::MaterialID> =
        //     zip(["hair", "cloth", "body", "head"], ina_textures)
        //         .map(|(id, tex)| {
        //             data.add_material(
        //                 phong_id,
        //                 id,
        //                 Some(data.get_pipeline(phong_id).create_material_set(
        //                     &renderer.allocators,
        //                     2,
        //                     tex,
        //                     linear_sampler.clone(),
        //                 )),
        //             )
        //         })
        //         .collect();

        // //  uv
        // let uv_mat_id = data.add_material(uv_id, "uv", None);

        // // meshes
        // //      suzanne
        // let Mesh(vertices, indices) = Mesh::from_obj(Path::new("models/suzanne.obj"))
        //     .pop()
        //     .unwrap();
        // let suzanne = Arc::new(Buffers::initialize_device_local(
        //     &renderer.allocators,
        //     renderer.queue.clone(),
        //     vertices,
        //     indices,
        // ));

        // //      square
        // let vertices = vec![
        //     VertexFull {
        //         position: [-0.25, -0.25, 0.0],
        //         normal: [0.0, 0.0, 1.0],
        //         colour: [0.0, 1.0, 0.0],
        //         uv: [0.0, 0.0],
        //     },
        //     VertexFull {
        //         position: [0.25, -0.25, 0.0],
        //         normal: [0.0, 0.0, 1.0],
        //         colour: [0.0, 1.0, 0.0],
        //         uv: [1.0, 0.0],
        //     },
        //     VertexFull {
        //         position: [-0.25, 0.25, 0.0],
        //         normal: [0.0, 0.0, 1.0],
        //         colour: [0.0, 1.0, 0.0],
        //         uv: [0.0, 1.0],
        //     },
        //     VertexFull {
        //         position: [0.25, 0.25, 0.0],
        //         normal: [0.0, 0.0, 1.0],
        //         colour: [0.0, 1.0, 0.0],
        //         uv: [1.0, 1.0],
        //     },
        // ];
        // let indices = vec![0, 1, 2, 2, 1, 3];
        // let square = Arc::new(Buffers::initialize_device_local(
        //     &renderer.allocators,
        //     renderer.queue.clone(),
        //     vertices,
        //     indices,
        // ));

        // //      lost empire
        // let le_meshes: Vec<Arc<Buffers<VertexFull>>> =
        //     Mesh::from_obj(Path::new("models/lost_empire.obj"))
        //         .into_iter()
        //         .map(|Mesh(vertices, indices)| {
        //             Arc::new(Buffers::initialize_device_local(
        //                 &renderer.allocators,
        //                 renderer.queue.clone(),
        //                 vertices,
        //                 indices,
        //             ))
        //         })
        //         .collect();

        // //      ina
        // let ina_meshes: Vec<Arc<Buffers<VertexFull>>> =
        //     Mesh::from_obj(Path::new("models/ina/ReadyToRigINA.obj"))
        //         .into_iter()
        //         .skip(2)
        //         .map(|Mesh(vertices, indices)| {
        //             Arc::new(Buffers::initialize_device_local(
        //                 &renderer.allocators,
        //                 renderer.queue.clone(),
        //                 vertices,
        //                 indices,
        //             ))
        //         })
        //         .collect();

        // println!("[Rendering Data]");
        // println!("Lost empire mesh count: {}", le_meshes.len());
        // println!("Ina mesh count: {}", ina_meshes.len());

        // // objects
        // let mut render_objects = Vec::new();
        // //  Suzanne
        // render_objects.push(Arc::new(RenderObject::new(suzanne, uv_mat_id.clone())));

        // //  Squares
        // for (x, y, z) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
        //     let mut square_obj = RenderObject::new(square.clone(), uv_mat_id.clone());
        //     square_obj.update_transform([x as f32, y as f32, z as f32], cgmath::Rad(0.));

        //     render_objects.push(Arc::new(square_obj));
        // }

        // //  Ina
        // for (mesh, mat_id) in zip(ina_meshes, ina_ids.clone()) {
        //     let mut obj = RenderObject::new(mesh, mat_id);
        //     obj.update_transform([0.0, 5.0, -1.0], cgmath::Rad(0.));

        //     render_objects.push(Arc::new(obj));
        // }

        // //  lost empires
        // for mesh in le_meshes {
        //     render_objects.push(Arc::new(RenderObject::new(mesh, le_mat_id.clone())));
        // }

        data
    }
}
