use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use cgmath::{Matrix4, Vector3, Vector4};
use legion::*;

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
};

use crate::{
    game_objects::{
        light::PointLightComponent,
        transform::{TransformCreateInfo, TransformID},
        DisabledLERP, GameWorld,
    },
    init_phys_test, init_ui_test, init_world,
    render::{
        renderer::DeferredRenderer, resource_manager::ResourceManager, RenderLoop, RenderObject,
    },
    shaders::{draw::GPUGlobalData, lighting::DirectionLight},
    ui::{self, MenuOption},
    MaterialSwapper, RENDER_PROFILER,
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
    mouse_dx: f32,
    mouse_dy: f32,
    // esc_triggered: bool,
}

impl InputState {
    fn get_move(&self) -> Vector3<f32> {
        let mut movement = <Vector3<f32> as cgmath::Zero>::zero();
        if self.w == Pressed {
            movement.z -= 1.; // forward
        } else if self.s == Pressed {
            movement.z += 1.; // backwards
        }
        if self.a == Pressed {
            movement.x -= 1.; // left
        } else if self.d == Pressed {
            movement.x += 1.; // right
        }
        if self.space == Pressed {
            movement.y += 1.;
        } else if self.shift == Pressed {
            movement.y -= 1.;
        }

        movement
    }
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
    world: Arc<Mutex<GameWorld>>,
    game_thread: GameWorldThread,
    inputs: InputState,
    game_state: GameState,
    last_frame_time: Instant,
}

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD, SPACE and LSHIFT to move and Q to swap materials");

        let init_start_time = Instant::now();

        // let world = World::default();
        // let mut transforms = TransformSystem::new();
        let render_loop = RenderLoop::new(event_loop);
        let renderer = DeferredRenderer::new(&render_loop.context);
        let resources = ResourceManager::new(&render_loop.context);

        render_loop.context.gui.context().style_mut(ui::set_style);

        let render_init_elapse = init_start_time.elapsed().as_millis();

        // println!("[Renderer Info]\nLit shaders:");
        // for shader in &renderer.lit_draw_system.shaders {
        //     println!("{}", shader);
        // }
        // println!("Unlit shaders:");
        // for shader in &renderer.unlit_draw_system.shaders {
        //     println!("{}", shader);
        // }

        let world = Arc::new(Mutex::new(GameWorld::new()));
        let game_thread = GameWorldThread::new(world.clone());
        game_thread.set_paused(true);

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
            inputs: InputState::default(),
            world,
            game_thread,
            game_state: Default::default(),
            last_frame_time: Instant::now(),
        }
    }

    fn load_level(&mut self, id: i32) -> Result<(), String> {
        let load_start = Instant::now();
        let resources = &mut self.resources.begin_retrieving(
            &self.render_loop.context,
            &mut self.renderer.lit_draw_system,
            &mut self.renderer.unlit_draw_system,
        );

        let GameWorld {
            world,
            transforms,
            camera,
            ..
        } = &mut *self.world.lock().unwrap();
        println!(
            "[Benchmarking] retrived world lock:    {} ms",
            load_start.elapsed().as_millis(),
        );
        let load_start = Instant::now();

        match id {
            0 => init_world(world, transforms, resources),
            1 => init_ui_test(world, transforms, resources),
            2 => init_phys_test(world, transforms, resources),
            _ => {
                return Err(format!("Tried to load invalid level id: {id}"));
            }
        }
        // camera light, child of the camera
        let camera_light = transforms.add_transform(
            TransformCreateInfo::default()
                .set_parent(Some(camera.transform))
                .set_translation((0., 0., -0.1)), // light pos cannot = cam pos else the light will glitch
        );

        world.push((
            camera_light,
            PointLightComponent {
                color: Vector4::new(1., 1., 1., 2.),
                half_radius: 4.,
            },
            // RenderObject::new(square, bill),
        ));

        let camera_child_1 = transforms.add_transform(
            TransformCreateInfo::default()
                .set_parent(Some(camera.transform))
                .set_translation((0., 0., -4.)), // light pos cannot = cam pos else the light will glitch
        );

        let camera_child_2 = transforms.add_transform(
            TransformCreateInfo::default()
                .set_parent(Some(camera.transform))
                .set_translation((0., 1., -4.)), // light pos cannot = cam pos else the light will glitch
        );

        let square = resources.get_mesh(crate::render::resource_manager::MeshID::Square);
        let red = resources.get_material(
            crate::render::resource_manager::MaterialID::Color([255, 0, 0, 255]),
            true,
        );
        let blue = resources.get_material(
            crate::render::resource_manager::MaterialID::Color([0, 0, 255, 255]),
            true,
        );
        world.push((camera_child_1, RenderObject::new(square.clone(), blue)));
        world.push((
            camera_child_2,
            RenderObject::new(square.clone(), red),
            DisabledLERP,
        ));

        println!(
            "[Benchmarking] level load time:        {} ms",
            load_start.elapsed().as_millis(),
        );

        Ok(())
    }

    pub fn handle_winit_event(
        &mut self,
        event: Event<()>,
        control_flow: &mut winit::event_loop::ControlFlow,
    ) {
        match event {
            Event::WindowEvent { event, .. } => {
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
            Event::RedrawRequested(_) => {
                let update_start = Instant::now();
                // let duration_from_last_frame = update_start - self.last_frame_time;

                // gui
                let mut gui_result = MenuOption::None;
                self.render_loop.context.gui.immediate_ui(|gui| {
                    let ctx = &gui.context();

                    ui::profiler_window(ctx);

                    // let window_rect = Rect::from_center_size((500., 300.).into(), Vec2::splat(200.));
                    match self.game_state {
                        GameState::MainMenu => ui::main_menu(ctx, &mut gui_result),
                        GameState::Paused => ui::pause_menu(ctx, &mut gui_result),
                        _ => {}
                    };
                });
                match gui_result {
                    ui::MenuOption::None => {}
                    ui::MenuOption::LoadLevel(i) => match self.load_level(i) {
                        Ok(()) => {
                            self.game_state = GameState::Playing;
                            self.lock_cursor();
                            self.game_thread.set_paused(false);
                        }
                        Err(e) => println!("[Error] {e}"),
                    },
                    ui::MenuOption::QuitLevel => {
                        self.game_state = GameState::MainMenu;
                        // self.unlock_cursor();

                        let mut world = self.world.lock().unwrap();
                        world.clear();
                    }
                    ui::MenuOption::Quit => control_flow.set_exit(),
                }

                // profile logic update
                unsafe {
                    let mut profiler = RENDER_PROFILER.take().unwrap();
                    profiler.add_sample(update_start.elapsed().as_micros() as u32, 0);
                    RENDER_PROFILER = Some(profiler);
                }

                self.update_render();

                unsafe {
                    let mut profiler = RENDER_PROFILER.take().unwrap();
                    profiler.end_frame();
                    RENDER_PROFILER = Some(profiler);
                }

                self.last_frame_time = update_start;
            }
            Event::MainEventsCleared => self.render_loop.context.window.request_redraw(),
            Event::DeviceEvent {
                event: winit::event::DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if self.game_state == GameState::Playing {
                    self.inputs.mouse_dx += delta.0 as f32;
                    self.inputs.mouse_dy += delta.1 as f32;
                }
            }
            _ => (),
        }
    }

    /// upload render objects and do render loop
    fn update_render(&mut self) {
        {
            let GameWorld {
                world,
                transforms,
                camera,
                inputs,
                ..
            } = &mut *self.world.lock().unwrap();

            // sync inputs
            if self.game_state == GameState::Playing {
                inputs.movement = self.inputs.get_move();
                // world.update(duration_from_last_frame.as_secs_f32());
            }

            println!(
                "[debug] interpolation: {}",
                transforms.update_interpolation()
            );

            // update mat swap
            if self.inputs.q_triggered {
                let mut query = <(&mut MaterialSwapper, &mut RenderObject<Matrix4<f32>>)>::query();

                query.for_each_mut(world, |(swapper, render_object)| {
                    let next_mat = swapper.swap_material();
                    // println!("Swapped mat: {:?}", next_mat);
                    render_object.material = next_mat;
                });

                self.inputs.q_triggered = false;
            }

            // update render objects
            let mut query = <(&TransformID, &mut RenderObject<Matrix4<f32>>)>::query()
                .filter(!component::<DisabledLERP>());
            // println!("==== RENDER OBJECT DATA ====");
            for (transform_id, render_object) in query.iter_mut(world) {
                let transfrom_matrix = transforms.get_lerp_model(transform_id).unwrap();
                // println!("Obj {:?}: {:?}", transform_id, obj);
                render_object.set_matrix(transfrom_matrix);
                render_object.upload();
            }

            // do not interpolate models
            let mut query =
                <(&TransformID, &mut RenderObject<Matrix4<f32>>, &DisabledLERP)>::query();
            for (transform_id, render_object, _) in query.iter_mut(world) {
                let transfrom_matrix = transforms.get_global_model(transform_id).unwrap();
                render_object.set_matrix(transfrom_matrix);
                render_object.upload();
            }

            // rotate camera
            let transform = transforms.get_transform_mut(&camera.transform).unwrap();
            transform.set_rotation(camera.rotate(
                transform.get_local_transform().rotation,
                self.inputs.mouse_dx,
                self.inputs.mouse_dy,
            ));

            self.inputs.mouse_dx = 0.;
            self.inputs.mouse_dy = 0.;
        }

        // do render loop
        let extends = self.render_loop.context.window.inner_size();
        self.render_loop
            .update(&mut self.renderer, |renderer, image_i| {
                let GameWorld {
                    world,
                    transforms,
                    camera,
                    fixed_seconds: total_seconds,
                    ..
                } = &mut *self.world.lock().unwrap();

                // camera data
                // let cam_transform = transforms
                //     .get_transform(&camera.transform)
                //     .unwrap()
                //     .get_local_transform();
                // println!("[debug] cam transform: {cam_transform:?}");
                let cam_model = transforms.get_lerp_model(&camera.transform).unwrap();
                let global_data = GPUGlobalData::from_camera(camera, cam_model, extends);

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
                let point_lights = point_query.iter(world).map(|(t, pl)| {
                    let pos = transforms.get_lerp_model(t).unwrap()[3];
                    pl.clone().into_light(pos.truncate() / pos.w)
                });
                frame.update_point_lights(point_lights);

                // directional lights
                // let mut dl_query = <(&TransformID, &DirectionalLightComponent)>::query();
                let angle = *total_seconds / 4.;
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

    /// update key state
    fn handle_keyboard_input(&mut self, key_code: VirtualKeyCode, state: ElementState) {
        let state = match state {
            ElementState::Pressed => Pressed,
            ElementState::Released => Released,
        };

        match key_code {
            VirtualKeyCode::Q => {
                self.inputs.q_triggered = state == Pressed && self.inputs.q == Released;
                self.inputs.q = state;
            }
            VirtualKeyCode::W => self.inputs.w = state,
            VirtualKeyCode::A => self.inputs.a = state,
            VirtualKeyCode::S => self.inputs.s = state,
            VirtualKeyCode::D => self.inputs.d = state,
            VirtualKeyCode::Space => self.inputs.space = state,
            VirtualKeyCode::LShift => self.inputs.shift = state,
            VirtualKeyCode::Escape => {
                // pause and unpause
                if state == Pressed && self.inputs.escape == Released {
                    match self.game_state {
                        GameState::Playing => {
                            self.game_state = GameState::Paused;
                            self.unlock_cursor();
                            self.game_thread.set_paused(true);
                        }
                        GameState::Paused => {
                            self.game_state = GameState::Playing;
                            self.lock_cursor();
                            self.game_thread.set_paused(false);
                        }
                        _ => {}
                    }
                };
                self.inputs.escape = state;
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

pub const FIXED_DELTA_TIME: f32 = 0.1;

struct GameWorldThread {
    thread: JoinHandle<()>,
    // delta_time: Duration,
    paused: Arc<AtomicBool>,
}

impl GameWorldThread {
    fn new(game_world: Arc<Mutex<GameWorld>>) -> Self {
        let paused = Arc::new(AtomicBool::new(false));
        let paused_2 = paused.clone();

        let thread = thread::spawn(move || {
            let update_period = Duration::from_secs_f32(FIXED_DELTA_TIME);
            let mut next_time = Instant::now() + update_period;
            // let mut last_update = Instant::now();
            loop {
                if !paused_2.load(std::sync::atomic::Ordering::Acquire) {
                    let wait = {
                        let update_start = Instant::now();
                        let mut world = game_world.lock().unwrap();
                        // let lock_wait = update_start.elapsed().as_millis();

                        // let pre_update = Instant::now();
                        world.update(FIXED_DELTA_TIME);
                        // let update_time = pre_update.elapsed().as_millis();

                        // skip frames if update took too long
                        let now = Instant::now();
                        let mut frames_skipped = 0;
                        while next_time < now {
                            next_time += update_period;
                            frames_skipped += 1;
                        }
                        if frames_skipped > 0 {
                            println!("[Warning] Skipped {frames_skipped} frames");
                        }
                        // println!(
                        //     "[Debug] Wait since last update: {:>4} ms, Set wait till next: {:>4} ms",
                        //     time_since_update, (next_time - now).as_millis()
                        // );

                        next_time - now
                    };
                    next_time += update_period;
                    thread::sleep(wait)
                } else {
                    thread::park();
                };
            }
        });

        Self { thread, paused }
    }

    fn set_paused(&self, paused: bool) {
        if paused {
            self.paused
                .store(true, std::sync::atomic::Ordering::Release);
        } else {
            self.paused
                .store(false, std::sync::atomic::Ordering::Release);
            self.thread.thread().unpark();
        }
    }
}
