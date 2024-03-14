use winit::event_loop::EventLoop;

use vulkano_template::app::App;
// use winit::keyboard::PhysicalKey;

fn main() {
    let event_loop = EventLoop::new(); //.unwrap();
    let mut app = App::start(&event_loop);

    // print!("\n\n\n\n\n\n\n\n");

    event_loop.run(move |event, _elwt, control_flow| {
        app.handle_winit_event(event, control_flow);
    })
    // .unwrap();
}
