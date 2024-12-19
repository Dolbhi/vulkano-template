use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use cgmath::{One, Quaternion, Vector3, Vector4};
use legion::*;

use rand::Rng;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
};

use crate::{
    game_objects::{
        light::PointLightComponent,
        transform::{TransformCreateInfo, TransformID},
        Camera, GameWorld, MaterialSwapper, WorldLoader,
    },
    physics::CuboidCollider,
    prefabs::{init_phys_test, init_ui_test, init_world},
    render::{resource_manager::ResourceManager, DeferredRenderer, RenderLoop, RenderObject},
    shaders::{DirectionLight, GPUGlobalData, GPUAABB},
    ui::{self, MenuOption},
    LOGIC_PROFILER, RENDER_PROFILER,
};

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
    r: KeyState,
    o: KeyState,
    p: KeyState,
    space: KeyState,
    shift: KeyState,
    escape: KeyState,
    q_triggered: bool,
    o_triggered: bool,
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
    camera_rotation: Quaternion<f32>,
    game_thread: GameWorldThread,
    inputs: InputState,
    game_state: GameState,
    last_frame_time: Instant,
    current_level: i32,
    bounds_debug_depth: Option<usize>,
}

const FIXED_DELTA_TIME: f32 = 0.02;
/// Struct for handling the logic thread of the game world
struct GameWorldThread {
    thread: JoinHandle<()>,
    delta_micros: Arc<AtomicU64>,
    paused: Arc<AtomicBool>,
}

// struct GameDataLoader<'a> {
//     global_data: GPUGlobalData,
//     ambient_light: [f32; 4],
//     world: &'a mut World,
//     transforms: &'a mut TransformSystem,
// }

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD, SPACE and LSHIFT to move and Q to swap materials");
        println!(
            "Press O to add a random bounding box to the BVH, press P to filter the depth shown"
        );
        println!("[TODO] Press F to toggle camera light");

        let init_start_time = Instant::now();

        let render_loop = RenderLoop::new(event_loop);
        let renderer = DeferredRenderer::new(&render_loop.context);
        let resources = ResourceManager::new(&render_loop.context);

        render_loop.context.gui.context().style_mut(ui::set_style);

        let render_init_elapse = init_start_time.elapsed().as_millis();

        let world = Arc::new(Mutex::new(GameWorld::new()));
        let game_thread = GameWorldThread::new(world.clone());
        game_thread.set_paused(true);

        println!("[Benchmarking] render init: {} ms", render_init_elapse,);

        Self {
            render_loop,
            renderer,
            resources,
            inputs: InputState::default(),
            world,
            camera_rotation: Quaternion::one(),
            game_thread,
            game_state: Default::default(),
            last_frame_time: Instant::now(),
            current_level: -1,
            bounds_debug_depth: None,
        }
    }

    fn load_level(&mut self, id: i32) -> Result<(), String> {
        // let load_start = Instant::now();

        // println!(
        //     "[Benchmarking] retrived world lock:    {} ms",
        //     load_start.elapsed().as_millis(),
        // );
        let load_start = Instant::now();

        let loader = match id {
            0 => init_world,
            1 => init_ui_test,
            2 => init_phys_test,
            _ => {
                return Err(format!("Tried to load invalid level id: {id}"));
            }
        };

        let world = &mut *self.world.lock().unwrap();
        world.clear();
        let resources = &mut self
            .resources
            .begin_retrieving(&self.render_loop.context, &mut self.renderer);

        loader(WorldLoader { world, resources });

        // camera light, child of the camera
        let camera_light = world.transforms.add_transform(
            TransformCreateInfo::default()
                .set_parent(Some(world.camera.transform))
                .set_translation((0., 0., 0.2)), // light pos cannot = cam pos else the light will glitch
        );
        world
            .world
            .push((camera_light, PointLightComponent::new([1., 1., 1., 2.], 4.)));

        self.current_level = id;

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
                    Camera::camera_rotation(
                        &mut self.camera_rotation,
                        delta.0 as f32,
                        delta.1 as f32,
                    );
                }
            }
            _ => (),
        }
    }

    /// upload render objects and do render loop
    fn update_render(&mut self) {
        // do render loop
        let extends = self.render_loop.context.window.inner_size();
        self.render_loop
            .update(&mut self.renderer, |renderer, image_i| {
                let GameWorld {
                    world,
                    transforms,
                    colliders,
                    camera,
                    fixed_seconds,
                    last_delta_time,
                    inputs,
                    ..
                } = &mut *self.world.lock().unwrap();
                transforms.update_interpolation(*last_delta_time);

                // sync inputs
                if self.game_state == GameState::Playing {
                    inputs.movement = self.inputs.get_move();
                    camera.set_rotation(self.camera_rotation);
                }
                camera.sync_transform(transforms);

                // update basic mat swap
                if self.inputs.q_triggered {
                    let mut query = <(&mut MaterialSwapper<()>, &mut RenderObject<()>)>::query();

                    query.for_each_mut(world, |(swapper, render_object)| {
                        let next_mat = swapper.swap_material();
                        // println!("Swapped mat: {:?}", next_mat);
                        render_object.material = next_mat;
                    });

                    self.inputs.q_triggered = false;
                }

                // add random bounds
                if self.inputs.o_triggered {
                    let mut rng = rand::thread_rng();

                    let pos = Vector3::new(
                        rng.gen_range(-4.0..4.0),
                        rng.gen_range(-4.0..4.0),
                        rng.gen_range(-4.0..4.0),
                    );
                    let scale = Vector3::new(
                        rng.gen_range(0.0..2.0),
                        rng.gen_range(0.0..2.0),
                        rng.gen_range(0.0..2.0),
                    );

                    let transform =
                        transforms.add_transform(TransformCreateInfo::from(pos).set_scale(scale));
                    let collider = CuboidCollider::new(transforms, transform);

                    colliders.add(collider);

                    self.inputs.o_triggered = false;
                }

                // camera data
                // let cam_model = transforms.get_slerp_model(&camera.transform).unwrap();
                let global_data = GPUGlobalData::from_camera(camera, extends);

                // update basic render objects
                let mut query = <(&TransformID, &mut RenderObject<()>)>::query();
                // println!("==== RENDER OBJECT DATA ====");
                for (transform_id, render_object) in query.iter_mut(world) {
                    render_object.update_and_upload(transform_id, transforms);
                }

                let mut query = <(&TransformID, &mut RenderObject<Vector4<f32>>)>::query();
                // println!("==== RENDER COLORED DATA ====");
                for (transform_id, render_object) in query.iter_mut(world) {
                    render_object.update_and_upload(transform_id, transforms);
                }

                // upload draw data (make into renderer function)
                let frame = {
                    let frame = renderer
                        .frame_data
                        .get_mut(image_i)
                        .expect("Renderer should have a frame for every swapchain image");

                    frame.update_global_data(global_data);
                    frame.update_objects_data(
                        renderer
                            .lit_draw_system
                            .shaders
                            .values_mut()
                            .chain(renderer.unlit_draw_system.shaders.values_mut()),
                    );
                    frame.update_colored_data(
                        renderer
                            .lit_colored_system
                            .shaders
                            .values_mut()
                            .chain(renderer.unlit_colored_system.shaders.values_mut()),
                    );

                    frame
                };

                // bounding box
                if self.bounds_debug_depth == Some(colliders.tree_depth()) {
                    self.bounds_debug_depth = None;
                }

                if let Some(debug_depth) = self.bounds_debug_depth {
                    let mut bounding_boxes: Vec<GPUAABB> = colliders
                        .bounds_iter()
                        .filter(|(_, depth)| *depth == debug_depth)
                        .map(|(bounds, depth)| {
                            let mag = 1. / (depth as f32 + 1.);
                            let min_cast: [f32; 3] = bounds.min.into();
                            let max_cast: [f32; 3] = bounds.max.into();
                            GPUAABB {
                                min: min_cast.into(),
                                max: max_cast.into(),
                                color: [1., mag, mag, 1.],
                            }
                        })
                        .collect();

                    // show overlaps
                    for (coll_1, coll_2) in colliders.get_potential_overlaps() {
                        let bounds_1 = coll_1.get_bounds();
                        let bounds_2 = coll_2.get_bounds();

                        let centre = bounds_1.centre();
                        let min_cast: [f32; 3] = (centre - Vector3::new(0.1, 0.1, 0.1)).into();
                        let max_cast: [f32; 3] = (centre + Vector3::new(0.1, 0.1, 0.1)).into();
                        bounding_boxes.push(GPUAABB {
                            min: min_cast.into(),
                            max: max_cast.into(),
                            color: [0., 1., 0., 1.],
                        });

                        let centre = bounds_2.centre();
                        let min_cast: [f32; 3] = (centre - Vector3::new(0.1, 0.1, 0.1)).into();
                        let max_cast: [f32; 3] = (centre + Vector3::new(0.1, 0.1, 0.1)).into();
                        bounding_boxes.push(GPUAABB {
                            min: min_cast.into(),
                            max: max_cast.into(),
                            color: [0., 1., 0., 1.],
                        });
                    }

                    frame.upload_box_data(bounding_boxes.into_iter());
                } else {
                    let mut bounding_boxes: Vec<GPUAABB> = colliders
                        .bounds_iter()
                        .map(|(bounds, depth)| {
                            let mag = 1. / (depth as f32 + 1.);
                            let min_cast: [f32; 3] = bounds.min.into();
                            let max_cast: [f32; 3] = bounds.max.into();
                            GPUAABB {
                                min: min_cast.into(),
                                max: max_cast.into(),
                                color: [1., mag, mag, 1.],
                            }
                        })
                        .collect();

                    // show overlaps
                    for (coll_1, coll_2) in colliders.get_potential_overlaps() {
                        let bounds_1 = coll_1.get_bounds();
                        let bounds_2 = coll_2.get_bounds();

                        let centre = bounds_1.centre();
                        let min_cast: [f32; 3] = (centre - Vector3::new(0.1, 0.1, 0.1)).into();
                        let max_cast: [f32; 3] = (centre + Vector3::new(0.1, 0.1, 0.1)).into();
                        bounding_boxes.push(GPUAABB {
                            min: min_cast.into(),
                            max: max_cast.into(),
                            color: [0., 1., 0., 1.],
                        });

                        let centre = bounds_2.centre();
                        let min_cast: [f32; 3] = (centre - Vector3::new(0.1, 0.1, 0.1)).into();
                        let max_cast: [f32; 3] = (centre + Vector3::new(0.1, 0.1, 0.1)).into();
                        bounding_boxes.push(GPUAABB {
                            min: min_cast.into(),
                            max: max_cast.into(),
                            color: [0., 1., 0., 1.],
                        });
                    }

                    frame.upload_box_data(bounding_boxes.into_iter());
                }

                // point lights
                let mut point_query = <(&TransformID, &PointLightComponent)>::query();
                let point_lights = point_query.iter(world).map(|(t, pl)| {
                    let pos = transforms.get_lerp_model(t).unwrap()[3];
                    pl.clone().into_light(pos.truncate() / pos.w)
                });
                frame.update_point_lights(point_lights);

                // directional lights
                // let mut dl_query = <(&TransformID, &DirectionalLightComponent)>::query();
                let angle = *fixed_seconds / 4.;
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
            VirtualKeyCode::R => {
                if self.game_state == GameState::Playing
                    && state == Pressed
                    && self.inputs.r == Released
                {
                    let _ = self.load_level(self.current_level);
                }
                self.inputs.r = state;
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
            VirtualKeyCode::O => {
                // add bounding box
                self.inputs.o_triggered = state == Pressed && self.inputs.o == Released;
                self.inputs.o = state;
            }
            VirtualKeyCode::P => {
                // scroll through depths
                if state == Pressed && self.inputs.p == Released {
                    if let Some(depth) = self.bounds_debug_depth {
                        self.bounds_debug_depth = Some(depth + 1);
                    } else {
                        self.bounds_debug_depth = Some(0);
                    }
                }
                self.inputs.p = state;
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

impl GameWorldThread {
    fn new(game_world: Arc<Mutex<GameWorld>>) -> Self {
        let paused = Arc::new(AtomicBool::new(false));
        let thread_paused = paused.clone();

        let mircos = (FIXED_DELTA_TIME * 1_000_000.0) as u64;
        let delta_micros = Arc::new(AtomicU64::new(mircos));
        let thread_micros = delta_micros.clone();

        let thread = thread::spawn(move || {
            let mut update_period = Duration::from_micros(mircos);
            let mut next_time = Instant::now() + update_period;
            // let mut last_update = Instant::now();
            loop {
                if !thread_paused.load(std::sync::atomic::Ordering::Relaxed) {
                    {
                        let update_start = std::time::Instant::now();

                        let new_micros = thread_micros.load(std::sync::atomic::Ordering::Relaxed);
                        let mut world = game_world.lock().unwrap();
                        // let lock_wait = update_start.elapsed().as_millis();

                        // [Profiling] Lock Wait
                        unsafe {
                            let mut profiler = LOGIC_PROFILER.lock().unwrap();
                            profiler.add_sample(update_start.elapsed().as_micros() as u32, 0);
                        }

                        world.update(update_period.as_secs_f32());
                        update_period = Duration::from_micros(new_micros);

                        // skip frames if update took too long
                        let now = Instant::now();
                        let mut frames_skipped = 0u32;
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
                    };
                    thread::sleep(next_time - Instant::now());
                    next_time += update_period;
                } else {
                    thread::park();
                    next_time = Instant::now();
                };
            }
        });

        Self {
            thread,
            delta_micros,
            paused,
        }
    }

    fn set_paused(&self, paused: bool) {
        self.paused
            .store(paused, std::sync::atomic::Ordering::Relaxed);
        if !paused {
            self.thread.thread().unpark();
        }
    }

    #[allow(dead_code)]
    fn set_delta_time(&self, micros: u64) {
        self.delta_micros
            .store(micros, std::sync::atomic::Ordering::Relaxed);
    }
}
