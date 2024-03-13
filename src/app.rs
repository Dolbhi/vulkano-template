use std::time::Instant;

use cgmath::{Matrix4, Quaternion, Rotation3, Vector3, Vector4};
use crossterm::QueueableCommand;
use legion::{IntoQuery, *};

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
};

use crate::{
    game_objects::{
        light::PointLightComponent,
        transform::{TransformID, TransformSystem},
        Camera, FollowCamera, Rotate,
    },
    init_ui_test, init_world,
    render::{
        renderer::DeferredRenderer, resource_manager::ResourceManager, RenderLoop, RenderObject,
    },
    shaders::{draw::GPUGlobalData, lighting::DirectionLight},
    ui::{self, MenuOption},
    MaterialSwapper,
};

// TO flush_next_future METHOD ADD PARAMS FOR PASSING CAMERA DESCRIPTOR SET

// pub enum UpdateResult {
//     None,
//     Quit,
// }

#[derive(Default, PartialEq)]
enum KeyState {
    Pressed,
    #[default]
    Released,
}

use KeyState::{Pressed, Released};

#[derive(Default)]
struct InputState {
    a: KeyState,
    w: KeyState,
    s: KeyState,
    d: KeyState,
    q: KeyState,
    space: KeyState,
    shift: KeyState,
    escape: KeyState,
    q_triggered: bool,
    // esc_triggered: bool,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum GameState {
    #[default]
    MainMenu,
    Playing,
    Paused,
}

pub struct App {
    render_loop: RenderLoop,
    renderer: DeferredRenderer,
    resources: ResourceManager,
    transforms: TransformSystem,
    world: World,
    camera: Camera,
    keys: InputState,
    total_seconds: f32,
    game_state: GameState,
    last_frame_time: Instant,
}

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD, SPACE and LSHIFT to move and Q to swap materials");

        let init_start_time = Instant::now();

        let world = World::default();
        let transforms = TransformSystem::new();
        let render_loop = RenderLoop::new(event_loop);
        let renderer = DeferredRenderer::new(&render_loop.context);
        let resources = ResourceManager::new(&render_loop.context);

        render_loop.context.gui.context().style_mut(ui::set_style);

        let render_init_elapse = init_start_time.elapsed().as_millis();

        println!("[Renderer Info]\nLit shaders:");
        for shader in &renderer.lit_draw_system.shaders {
            println!("{}", shader);
        }
        println!("Unlit shaders:");
        for shader in &renderer.unlit_draw_system.shaders {
            println!("{}", shader);
        }

        println!(
            "[Benchmarking] render init: {} ms",
            render_init_elapse,
            // total_elapse - render_init_elapse,
            // total_elapse
        );

        Self {
            render_loop,
            renderer,
            resources,
            camera: Default::default(),
            keys: InputState::default(),
            world,
            transforms,
            total_seconds: 0.,
            game_state: Default::default(),
            last_frame_time: Instant::now(),
        }
    }

    fn load_level(&mut self, id: i32) {
        if id == 0 {
            init_world(
                &mut self.world,
                &mut self.transforms,
                &mut self.resources.begin_retrieving(
                    &self.render_loop.context,
                    &mut self.renderer.lit_draw_system,
                    &mut self.renderer.unlit_draw_system,
                ),
            );
        } else if id == 1 {
            init_ui_test(
                &mut self.world,
                &mut self.transforms,
                &mut self.resources.begin_retrieving(
                    &self.render_loop.context,
                    &mut self.renderer.lit_draw_system,
                    &mut self.renderer.unlit_draw_system,
                ),
            );
        }

        // camera light, will follow camera position on update
        let camera_light = self.transforms.next().unwrap();
        self.world.push((
            camera_light,
            PointLightComponent {
                color: Vector4::new(1., 1., 1., 2.),
                half_radius: 4.,
            },
            FollowCamera(Vector3::new(0., 0.01, 0.01)), // light pos cannot = cam pos else the light will glitch
        ));

        self.game_state = GameState::Playing;
    }

    pub fn handle_winit_event(
        &mut self,
        event: Event<()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    ) {
        match (self.game_state, event) {
            (_, Event::WindowEvent { event, .. }) => {
                if !self.render_loop.context.gui.update(&event) {
                    match event {
                        WindowEvent::CloseRequested => control_flow.set_exit(),
                        WindowEvent::Resized(_) => self.render_loop.handle_window_resize(),
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(code),
                                    state,
                                    ..
                                },
                            ..
                        } => self.handle_keyboard_input(code, state),

                        _ => {}
                    }
                }
            }
            (_, Event::RedrawRequested(_)) => {
                std::io::stdout()
                    .queue(crossterm::cursor::MoveToPreviousLine(8))
                    .unwrap();

                let update_start = Instant::now();
                let duration_from_last_frame = update_start - self.last_frame_time;

                // gui
                let mut gui_result = MenuOption::None;
                self.render_loop.context.gui.immediate_ui(|gui| {
                    let ctx = &gui.context();

                    ui::test_window(ctx);

                    // let window_rect = Rect::from_center_size((500., 300.).into(), Vec2::splat(200.));
                    match self.game_state {
                        GameState::MainMenu => ui::main_menu(ctx, &mut gui_result),
                        GameState::Paused => ui::pause_menu(ctx, &mut gui_result),
                        _ => {}
                    };
                });
                match gui_result {
                    ui::MenuOption::None => {}
                    ui::MenuOption::LoadLevel(i) => {
                        self.lock_cursor();
                        self.load_level(i);
                    }
                    ui::MenuOption::QuitLevel => {
                        self.unlock_cursor();

                        self.game_state = GameState::MainMenu;
                        self.world.clear();
                        self.transforms = TransformSystem::new();
                        self.camera = Default::default();
                    }
                    ui::MenuOption::Quit => control_flow.set_exit(),
                }

                if self.game_state == GameState::Playing {
                    self.update_game(duration_from_last_frame.as_secs_f32());
                }

                println!(
                    "\rLogic update    {:>4} μs",
                    update_start.elapsed().as_micros()
                );

                self.update_render();

                let elapsed = update_start.elapsed().as_micros();
                println!(
                    "\rTotal           {:>4} μs ({} fps)    ",
                    elapsed,
                    1_000_000 / elapsed
                );

                self.last_frame_time = update_start;
            }
            (_, Event::MainEventsCleared) => self.render_loop.context.window.request_redraw(),
            (
                GameState::Playing,
                Event::DeviceEvent {
                    event: winit::event::DeviceEvent::MouseMotion { delta },
                    ..
                },
            ) => self.camera.rotate(delta.0 as f32, delta.1 as f32),
            _ => (),
        }
    }

    /// update game logic
    fn update_game(&mut self, seconds_passed: f32) {
        self.total_seconds += seconds_passed;

        // move cam
        self.update_movement(seconds_passed);

        // update camera follower
        let mut query = <(&TransformID, &FollowCamera)>::query();
        for (transform_id, &FollowCamera(offset)) in query.iter(&self.world) {
            self.transforms
                .get_transform_mut(transform_id)
                .unwrap()
                .set_translation(self.camera.position + offset);
        }

        // update rotate
        let mut query = <(&TransformID, &Rotate)>::query();
        for (transform_id, rotate) in query.iter(&self.world) {
            let transform = self.transforms.get_transform_mut(transform_id).unwrap();
            transform.set_rotation(
                Quaternion::from_axis_angle(rotate.0, rotate.1 * seconds_passed)
                    * transform.get_local_transform().rotation,
            );
        }

        // update mat swap
        if self.keys.q_triggered {
            let mut query = <(&mut MaterialSwapper, &mut RenderObject<Matrix4<f32>>)>::query();

            query.for_each_mut(&mut self.world, |(swapper, render_object)| {
                let next_mat = swapper.swap_material();
                // println!("Swapped mat: {:?}", next_mat);
                render_object.material = next_mat;
            });

            self.keys.q_triggered = false;
        }
    }

    /// upload render objects and do render loop
    fn update_render(&mut self) {
        // update render objects
        let mut query = <(&TransformID, &mut RenderObject<Matrix4<f32>>)>::query();
        // println!("==== RENDER OBJECT DATA ====");
        for (transform_id, render_object) in query.iter_mut(&mut self.world) {
            let transfrom_matrix = self.transforms.get_global_model(transform_id).unwrap();
            // println!("Obj {:?}: {:?}", transform_id, obj);
            render_object.set_matrix(transfrom_matrix);
            render_object.upload();
        }

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

                frame.update_objects_data(
                    &mut renderer.lit_draw_system,
                    &mut renderer.unlit_draw_system,
                );

                // point lights
                let mut point_query = <(&TransformID, &PointLightComponent)>::query();
                let point_lights = point_query.iter(&self.world).map(|(t, pl)| {
                    pl.clone()
                        .into_light(self.transforms.get_global_position(t).unwrap())
                });
                frame.update_point_lights(point_lights);

                // directional lights
                // let mut dl_query = <(&TransformID, &DirectionalLightComponent)>::query();
                let angle = self.total_seconds / 4.;
                let direction =
                    cgmath::InnerSpace::normalize(cgmath::vec3(angle.sin(), -1., angle.cos()));
                let dir = DirectionLight {
                    color: [0.5, 0.5, 0., 1.],
                    direction: direction.extend(1.).into(),
                };
                frame.update_directional_lights([dir].into_iter());

                // ambient light
                renderer
                    .lighting_system
                    .set_ambient_color([0.1, 0.1, 0.1, 1.]);
            });
    }

    /// move camera
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

    /// update key state
    fn handle_keyboard_input(&mut self, key_code: VirtualKeyCode, state: ElementState) {
        let state = match state {
            ElementState::Pressed => Pressed,
            ElementState::Released => Released,
        };

        match key_code {
            VirtualKeyCode::Q => {
                self.keys.q_triggered = state == Pressed && self.keys.q == Released;
                self.keys.q = state;
            }
            VirtualKeyCode::W => self.keys.w = state,
            VirtualKeyCode::A => self.keys.a = state,
            VirtualKeyCode::S => self.keys.s = state,
            VirtualKeyCode::D => self.keys.d = state,
            VirtualKeyCode::Space => self.keys.space = state,
            VirtualKeyCode::LShift => self.keys.shift = state,
            VirtualKeyCode::Escape => {
                // pause and unpause
                if state == Pressed && self.keys.escape == Released {
                    match self.game_state {
                        GameState::Playing => {
                            self.game_state = GameState::Paused;

                            self.unlock_cursor();
                        }
                        GameState::Paused => {
                            self.game_state = GameState::Playing;

                            self.lock_cursor();
                        }
                        _ => {}
                    }
                };
                self.keys.escape = state;
            }
            _ => {}
        }
    }

    fn lock_cursor(&mut self) {
        let window = &self.render_loop.context.window;
        window.set_cursor_visible(false);
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .or_else(|_e| window.set_cursor_grab(winit::window::CursorGrabMode::Locked))
            .unwrap();
    }
    fn unlock_cursor(&mut self) {
        let window = &self.render_loop.context.window;
        let window_size = window.inner_size();
        window
            .set_cursor_position(PhysicalPosition::new(
                window_size.width / 2,
                window_size.height / 2,
            ))
            .unwrap();
        window.set_cursor_visible(true);
        window
            .set_cursor_grab(winit::window::CursorGrabMode::None)
            .unwrap();
    }
}
