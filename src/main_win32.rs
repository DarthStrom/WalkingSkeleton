use winapi::{
    shared::windef::{HBITMAP, HDC, HGDIOBJ, HWND, RECT},
    um::{
        wingdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteObject, StretchDIBits, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD, SRCCOPY,
        },
        winuser,
    },
};
use winit::{
    os::windows::WindowExt, ControlFlow, Event::WindowEvent, WindowBuilder, WindowEvent::*,
};

const WINDOW_NAME: &str = "Walking Skeleton";
static mut BITMAP_INFO: BITMAPINFO = BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER {
        biSize: 0,
        biWidth: 0,
        biHeight: 0,
        biPlanes: 0,
        biBitCount: 0,
        biCompression: 0,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    },
    bmiColors: [RGBQUAD {
        rgbBlue: 0,
        rgbGreen: 0,
        rgbRed: 0,
        rgbReserved: 0,
    }; 1],
};
static mut BITMAP_MEMORY: *mut std::ffi::c_void = std::ptr::null_mut();
static mut BITMAP_HANDLE: HBITMAP = std::ptr::null_mut();
static mut BITMAP_DEVICE_CONTEXT: HDC = std::ptr::null_mut();

unsafe fn resize_dib_section(width: i32, height: i32) {
    if !BITMAP_HANDLE.is_null() {
        DeleteObject(BITMAP_HANDLE as HGDIOBJ);
    }

    if BITMAP_DEVICE_CONTEXT.is_null() {
        BITMAP_DEVICE_CONTEXT = CreateCompatibleDC(std::ptr::null_mut());
    }

    BITMAP_INFO.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as _;
    BITMAP_INFO.bmiHeader.biWidth = width;
    BITMAP_INFO.bmiHeader.biHeight = height;
    BITMAP_INFO.bmiHeader.biPlanes = 1;
    BITMAP_INFO.bmiHeader.biBitCount = 32;
    BITMAP_INFO.bmiHeader.biCompression = BI_RGB;

    BITMAP_HANDLE = CreateDIBSection(
        BITMAP_DEVICE_CONTEXT,
        &BITMAP_INFO as _,
        DIB_RGB_COLORS,
        &mut BITMAP_MEMORY,
        std::ptr::null_mut(),
        0,
    );
}

unsafe fn update_window(device_context: HDC, x: i32, y: i32, width: i32, height: i32) {
    StretchDIBits(
        device_context,
        x,
        y,
        width,
        height,
        x,
        y,
        width,
        height,
        BITMAP_MEMORY,
        &BITMAP_INFO,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

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
            println!("wm_size: {:?}", logical_size);

            let hwnd = window.get_hwnd() as HWND;
            let mut client_rect = RECT::default();
            unsafe {
                winuser::GetClientRect(hwnd, &mut client_rect);
                let width = client_rect.right - client_rect.left;
                let height = client_rect.bottom - client_rect.top;
                resize_dib_section(width, height);
            }

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
                update_window(device_context, x, y, width, height);
                winuser::EndPaint(hwnd, &paint);
            }

            ControlFlow::Continue
        }
        _ => ControlFlow::Continue,
    });
}
