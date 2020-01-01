use winapi::{
    shared::windef::HWND,
    um::{
        wingdi::{PatBlt, WHITENESS},
        winuser,
    },
};
use winit::{
    os::windows::WindowExt, ControlFlow, Event::WindowEvent, WindowBuilder, WindowEvent::*,
};

const WINDOW_NAME: &str = "Walking Skeleton";

pub fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let window = WindowBuilder::new()
        .with_title(WINDOW_NAME)
        .build(&events_loop)
        .expect("could not create window");

    events_loop.run_forever(|event| match event {
        // wm_size
        WindowEvent {
            event: Resized(logical_size),
            ..
        } => {
            // TODO: are you supposed to repaint here as well?
            // when the window is resized we don't seem to be
            // getting a refresh event
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
        WindowEvent { event: Refresh, .. } => {
            let mut paint = winuser::PAINTSTRUCT::default();
            let hwnd = window.get_hwnd() as HWND;
            unsafe {
                let device_context = winuser::BeginPaint(hwnd, &mut paint);
                let x = paint.rcPaint.left;
                let y = paint.rcPaint.top;
                let width = paint.rcPaint.right - paint.rcPaint.left;
                let height = paint.rcPaint.bottom - paint.rcPaint.top;
                PatBlt(device_context, x, y, width, height, WHITENESS);
                winuser::EndPaint(hwnd, &paint);
            }

            ControlFlow::Continue
        }
        _ => ControlFlow::Continue,
    });
}
