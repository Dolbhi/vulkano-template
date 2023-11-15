use std::time::Duration;

use vulkano_template::game_objects::Camera;
use winit::event::{ElementState, VirtualKeyCode};
use winit::event_loop::EventLoop;

use crate::render::RenderLoop;

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
    space: KeyState,
    shift: KeyState,
}

pub struct App {
    render_loop: RenderLoop,
    camera: Camera,
    keys: Keys,
}

impl App {
    pub fn start(event_loop: &EventLoop<()>) -> Self {
        println!("Welcome to THE RUSTY RENDERER!");
        println!("Press WASD to move and SPACE to change color");

        Self {
            render_loop: RenderLoop::new(event_loop),
            camera: Camera::default(),
            keys: Keys::default(),
        }
    }

    pub fn update(&mut self, duration_since_last_update: &Duration) {
        let seconds_passed = (duration_since_last_update.as_micros() as f32) / 1000000.0;
        // println!("Current time: {}", seconds_passed);

        self.update_movement(seconds_passed);

        self.render_loop.update(&self.camera, seconds_passed);
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

    pub fn handle_keyboard_input(&mut self, key_code: VirtualKeyCode, state: ElementState) {
        let state = match state {
            ElementState::Pressed => Pressed,
            ElementState::Released => Released,
        };

        match key_code {
            // VirtualKeyCode::Space => {
            //     if state == Pressed && self.keys.space == Released {
            //         // self.square.change_to_random_color();
            //     }
            //     self.keys.space = state;
            // }
            VirtualKeyCode::W => self.keys.w = state,
            VirtualKeyCode::A => self.keys.a = state,
            VirtualKeyCode::S => self.keys.s = state,
            VirtualKeyCode::D => self.keys.d = state,
            VirtualKeyCode::Space => self.keys.space = state,
            VirtualKeyCode::LShift => self.keys.shift = state,
            _ => {}
        }
    }

    pub fn handle_window_resize(&mut self) {
        self.render_loop.handle_window_resize()
    }
}
