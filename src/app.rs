use std::time::Duration;
use std::{path::Path, sync::Arc};

use cgmath::{Euler, Matrix4, Rad, Vector3, Vector4};
use legion::{IntoQuery, *};
use vulkano::buffer::BufferUsage;
use vulkano::descriptor_set::WriteDescriptorSet;
use winit::{event::ElementState, event_loop::EventLoop, keyboard::KeyCode};

use crate::{
    game_objects::{
        light::PointLightComponent,
        transform::{TransformCreateInfo, TransformID, TransformSystem},
        Camera,
    },
    init_render_objects,
    render::{mesh::from_obj, renderer::DeferredRenderer, RenderLoop, RenderObject},
    shaders::{
        draw::{self, GPUGlobalData},
        lighting::DirectionLight,
    },
    MaterialSwapper,
};

// TO flush_next_future METHOD ADD PARAMS FOR PASSING CAMERA DESCRIPTOR SET

#[derive(Default, PartialEq)]
pub enum KeyState {
    Pressed,
    #[default]
    Released,
}

use KeyState::{Pressed, Released};

#[derive(Default)]
struct Keys {
    a: KeyState,
    w: KeyState,
    s: KeyState,
    d: KeyState,
    q: KeyState,
    space: KeyState,
    shift: KeyState,
}

pub struct App {
    render_loop: RenderLoop,
    renderer: DeferredRenderer,
    camera: Camera,
    keys: Keys,
    world: World,
    transforms: TransformSystem,
    total_seconds: f32,
    suzanne: TransformID,
    camera_light: TransformID,
    test_light: Arc<RenderObject<Matrix4<f32>>>,
}

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD, SPACE and LSHIFT to move and Q to swap materials");

        let mut world = World::default();
        let mut transforms = TransformSystem::new();
        let render_loop = RenderLoop::new(event_loop);
        let mut renderer = DeferredRenderer::new(&render_loop.context);

        // draw objects
        let suzanne = init_render_objects(
            &mut world,
            &mut transforms,
            &render_loop.context,
            &mut renderer.lit_draw_system,
        );
        // BIG RED TEST LIGHT
        let test_light = {
            let resource_loader = render_loop.context.get_resource_loader();
            let system = &mut renderer.unlit_draw_system;
            let solid_id = 0;

            // mesh
            let (vertices, indices) = from_obj(Path::new("models/default_cube.obj"))
                .pop()
                .expect("Failed to load cube mesh");
            let mesh = resource_loader.load_mesh(vertices, indices);

            // material
            let buffer = resource_loader.create_material_buffer(
                draw::SolidData {
                    color: [1., 0., 0., 1.],
                },
                BufferUsage::empty(),
            );
            let solid_pipeline = &mut system.pipelines[solid_id];
            let material_id = solid_pipeline.add_material(
                "red",
                Some(solid_pipeline.create_material_set(
                    &render_loop.context.allocators,
                    2,
                    [WriteDescriptorSet::buffer(0, buffer)],
                )),
            );

            let mut ro = RenderObject::new(mesh, material_id);
            ro.set_matrix(
                Matrix4::from_translation([0., 5., -1.].into()) * Matrix4::from_scale(0.2),
            );
            Arc::new(ro)
        };

        // lights
        world.push((
            transforms.add_transform(TransformCreateInfo {
                scale: Vector3::new(0.1, 0.1, 0.1),
                translation: Vector3::new(0., 5., -1.),
                ..Default::default()
            }),
            PointLightComponent {
                color: Vector4::new(1., 0., 0., 1.),
            },
        ));
        world.push((
            transforms.add_transform(TransformCreateInfo {
                scale: Vector3::new(0.1, 0.1, 0.1),
                translation: Vector3::new(0.0, 6.0, -1.0),
                ..Default::default()
            }),
            PointLightComponent {
                color: Vector4::new(0., 0., 1., 1.),
            },
        ));

        // camera light, will follow camera position on update
        let camera_light = {
            let camera_light = transforms.next().unwrap();
            world.push((
                camera_light,
                PointLightComponent {
                    color: Vector4::new(1., 1., 1., 1.),
                },
            ));
            camera_light
        };

        Self {
            render_loop,
            renderer,
            camera: Default::default(),
            keys: Keys::default(),
            world,
            transforms,
            total_seconds: 0.,
            suzanne,
            camera_light,
            test_light,
        }
    }

    pub fn update(&mut self, duration_since_last_update: &Duration) {
        let seconds_passed = duration_since_last_update.as_secs_f32();
        self.total_seconds += seconds_passed;
        // println!("Current time: {}", seconds_passed);

        // move cam
        self.update_movement(seconds_passed);
        self.transforms
            .get_transform_mut(&self.camera_light)
            .unwrap()
            .set_translation(self.camera.position + Vector3::new(0., 0.01, 0.01)); // light pos cannot = cam pos else the light will glitch

        // rotate suzanne
        self.transforms
            .get_transform_mut(&self.suzanne)
            .unwrap()
            .set_rotation(Euler {
                x: Rad(0.),
                y: Rad(self.total_seconds),
                z: Rad(0.),
            });

        // update render objects
        let mut query = <(&TransformID, &mut RenderObject<Matrix4<f32>>)>::query();
        // println!("==== RENDER OBJECT DATA ====");
        for (transform_id, render_object) in query.iter_mut(&mut self.world) {
            // update object data
            // match Arc::get_mut(render_object) {
            //     Some(obj) => {
            let transfrom_matrix = self.transforms.get_global_model(transform_id);
            // println!("Obj {:?}: {:?}", transform_id, obj);
            render_object.set_matrix(transfrom_matrix);
            render_object.upload();
            // obj.update_transform([0., 0., 0.], cgmath::Rad(self.total_seconds));
            //     }
            //     None => {
            //         panic!("Unable to update render object");
            //     }
            // }
        }

        // upload test light
        self.test_light.upload();

        // do render loop
        let extends = self.render_loop.context.window.inner_size();
        self.render_loop
            .update(&mut self.renderer, |renderer, image_i| {
                // camera data
                let global_data = GPUGlobalData::from_camera(&self.camera, extends);

                // upload draw data
                let frame = renderer
                    .frame_data
                    .get_mut(image_i)
                    .expect("Renderer should have a frame for every swapchain image");

                frame.update_global_data(global_data);

                frame.update_objects_data(&mut renderer.lit_draw_system);

                frame.update_unlit_data(&mut renderer.unlit_draw_system);

                // point lights
                let mut point_query = <(&TransformID, &PointLightComponent)>::query();
                let point_lights = point_query.iter(&self.world).map(|(t, pl)| {
                    pl.clone().into_light(
                        self.transforms
                            .get_transform(t)
                            .unwrap()
                            .get_transform()
                            .translation
                            .clone(),
                    )
                });
                frame.update_point_lights(point_lights);

                // directional lights
                // let mut dl_query = <(&TransformID, &DirectionalLightComponent)>::query();
                let angle = self.total_seconds / 4.;
                let direction =
                    cgmath::InnerSpace::normalize(cgmath::vec3(angle.sin(), -1., angle.cos()));
                let dir = DirectionLight {
                    color: [1., 1., 0., 1.],
                    direction: direction.extend(1.).into(),
                };
                frame.update_directional_lights([dir].into_iter());

                // ambient light
                renderer
                    .lighting_system
                    .set_ambient_color([0.2, 0.2, 0.2, 1.]);
            });
    }

    fn update_movement(&mut self, seconds_passed: f32) {
        if self.keys.space == Pressed && self.keys.shift == Released {
            self.camera.move_up(seconds_passed)
        }
        if self.keys.shift == Pressed && self.keys.space == Released {
            self.camera.move_down(seconds_passed)
        }

        if self.keys.w == Pressed && self.keys.s == Released {
            self.camera.move_forward(seconds_passed)
        }
        if self.keys.s == Pressed && self.keys.w == Released {
            self.camera.move_back(seconds_passed)
        }

        if self.keys.a == Pressed && self.keys.d == Released {
            self.camera.move_left(seconds_passed)
        }
        if self.keys.d == Pressed && self.keys.a == Released {
            self.camera.move_right(seconds_passed)
        }
    }

    pub fn handle_mouse_input(&mut self, dx: f32, dy: f32) {
        self.camera.rotate(dx, dy);

        // println!(
        //     "[Camera Rotation] x: {}, y: {}, z: {}",
        //     self.camera.rotation.x.0, self.camera.rotation.y.0, self.camera.rotation.z.0
        // );
    }
    pub fn handle_keyboard_input(&mut self, key_code: KeyCode, state: ElementState) {
        let state = match state {
            ElementState::Pressed => Pressed,
            ElementState::Released => Released,
        };

        match key_code {
            KeyCode::KeyQ => {
                if state == Pressed && self.keys.q == Released {
                    let mut query =
                        <(&mut MaterialSwapper, &mut Arc<RenderObject<Matrix4<f32>>>)>::query();

                    query.for_each_mut(&mut self.world, |(swapper, render_object)| {
                        let next_mat = swapper.swap_material();
                        // println!("Swapped mat: {:?}", next_mat);
                        match Arc::get_mut(render_object) {
                            Some(obj) => {
                                obj.material = next_mat;
                            }
                            None => {
                                panic!("Unable to swap material on render object");
                            }
                        }
                    });
                }
                self.keys.q = state;
            }
            KeyCode::KeyW => self.keys.w = state,
            KeyCode::KeyA => self.keys.a = state,
            KeyCode::KeyS => self.keys.s = state,
            KeyCode::KeyD => self.keys.d = state,
            KeyCode::Space => self.keys.space = state,
            KeyCode::ShiftLeft => self.keys.shift = state,
            _ => {}
        }
    }

    pub fn handle_window_resize(&mut self) {
        self.render_loop.handle_window_resize()
    }
    pub fn handle_window_wait(&self) {
        self.render_loop.context.window.request_redraw();
    }
}
