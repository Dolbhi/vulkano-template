use std::time::Instant;

use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::EventLoop;

use vulkano_template::app::App;
use winit::keyboard::PhysicalKey;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::start(&event_loop);

    let mut previous_frame_time = Instant::now();
    let mut window_focused = false;
    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                app.handle_window_resize();
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } => {
                window_focused = focused;
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                if window_focused {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        app.handle_keyboard_input(code, event.state);
                    }
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if window_focused {
                    app.handle_mouse_input(delta.0 as f32, delta.1 as f32);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let this_frame_time = Instant::now();
                let duration_from_last_frame = this_frame_time - previous_frame_time;

                app.update(&duration_from_last_frame);
                let elapsed = this_frame_time.elapsed().as_micros();
                print!("\rUpdate took {}μ ({}fps)   ", elapsed, 1_000_000 / elapsed);

                previous_frame_time = this_frame_time;
            }
            Event::AboutToWait => app.handle_window_wait(),
            _ => (),
        })
        .unwrap();
}
