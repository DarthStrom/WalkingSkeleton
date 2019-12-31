use winit::{ControlFlow, Event::WindowEvent, WindowEvent::*};

pub fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let _window = winit::Window::new(&events_loop).expect("could not create window");

    events_loop.run_forever(|event| match event {
        // wm_size
        WindowEvent {
            event: Resized(logical_size),
            ..
        } => {
            println!("wm_size: {:?}", logical_size);
            ControlFlow::Continue
        }
        // wm_destroy
        WindowEvent {
            event: Destroyed, ..
        } => {
            println!("wm_destroy");
            ControlFlow::Break
        }
        // wm_close
        WindowEvent {
            event: CloseRequested,
            ..
        } => {
            println!("wm_close");
            ControlFlow::Break
        }
        // wm_activateapp
        WindowEvent {
            event: Focused(gained_focus),
            ..
        } => {
            println!("wm_activateapp: gained focus: {}", gained_focus);
            ControlFlow::Continue
        }
        _ => {
            println!("default");
            ControlFlow::Continue
        }
    });
}
