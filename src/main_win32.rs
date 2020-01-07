use std::{ffi::OsStr, iter::once, mem::zeroed, os::windows::ffi::OsStrExt, ptr::null_mut};
use winapi::{
    ctypes::c_void,
    shared::{minwindef, windef},
    um::{
        libloaderapi,
        memoryapi::{VirtualAlloc, VirtualFree},
        wingdi::{
            StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD, SRCCOPY,
        },
        winnt::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE},
        winuser::*,
    },
};

const BYTES_PER_PIXEL: i32 = 4;
const WINDOW_NAME: &str = "Walking Skeleton";
const WINDOW_CLASS_NAME: &str = "WalkingSkeletonWindowClass";

static mut RUNNING: bool = true;
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
static mut BITMAP_MEMORY: *mut c_void = null_mut();

static mut BITMAP_WIDTH: i32 = 0;
static mut BITMAP_HEIGHT: i32 = 0;

unsafe fn render_weird_gradient(x_offset: i32, y_offset: i32) {
    let pitch = BITMAP_WIDTH * BYTES_PER_PIXEL;
    for y in 0..BITMAP_HEIGHT {
        let row = (BITMAP_MEMORY as *mut u8).offset((y * pitch) as isize);
        for x in 0..BITMAP_WIDTH {
            let pixel = row.offset((x * BYTES_PER_PIXEL) as isize);
            let blue = pixel;
            let green = pixel.offset(1);
            let red = pixel.offset(2);
            *red = 0;
            *green = (y + y_offset) as u8;
            *blue = (x + x_offset) as u8;
        }
    }
}

unsafe fn resize_dib_section(width: i32, height: i32) {
    if !BITMAP_MEMORY.is_null() {
        VirtualFree(BITMAP_MEMORY, 0, MEM_RELEASE);
    }

    BITMAP_WIDTH = width;
    BITMAP_HEIGHT = height;

    BITMAP_INFO.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as _;
    BITMAP_INFO.bmiHeader.biWidth = BITMAP_WIDTH;
    BITMAP_INFO.bmiHeader.biHeight = -BITMAP_HEIGHT;
    BITMAP_INFO.bmiHeader.biPlanes = 1;
    BITMAP_INFO.bmiHeader.biBitCount = 32;
    BITMAP_INFO.bmiHeader.biCompression = BI_RGB;

    let bitmap_memory_size = BYTES_PER_PIXEL * BITMAP_WIDTH * BITMAP_HEIGHT;
    BITMAP_MEMORY = VirtualAlloc(
        null_mut(),
        bitmap_memory_size as usize,
        MEM_RESERVE | MEM_COMMIT,
        PAGE_READWRITE,
    );
}

unsafe fn update_window(device_context: windef::HDC, client_rect: &windef::RECT) {
    let window_width = client_rect.right - client_rect.left;
    let window_height = client_rect.bottom - client_rect.top;

    StretchDIBits(
        device_context,
        0,
        0,
        BITMAP_WIDTH,
        BITMAP_HEIGHT,
        0,
        0,
        window_width,
        window_height,
        BITMAP_MEMORY,
        &BITMAP_INFO,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

unsafe extern "system" fn main_window_callback(
    window: windef::HWND,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> minwindef::LRESULT {
    let mut result = 0;
    let mut client_rect = windef::RECT::default();
    GetClientRect(window, &mut client_rect);

    match message {
        WM_ACTIVATEAPP => {}
        WM_CLOSE | WM_DESTROY => RUNNING = false,
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let device_context = BeginPaint(window, &mut paint);
            update_window(device_context, &client_rect);
            EndPaint(window, &paint);
        }
        WM_SIZE => {
            let width = client_rect.right - client_rect.left;
            let height = client_rect.bottom - client_rect.top;
            resize_dib_section(width, height);
        }
        _ => {
            result = DefWindowProcW(window, message, w_param, l_param);
        }
    }

    result
}

fn win32_string(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

pub fn main() {
    unsafe {
        let window_class = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(main_window_callback),
            hInstance: libloaderapi::GetModuleHandleW(null_mut()),
            lpszClassName: win32_string(WINDOW_CLASS_NAME).as_ptr(),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null_mut(),
        };

        if RegisterClassW(&window_class) > 0 {
            let window = CreateWindowExW(
                0,
                window_class.lpszClassName,
                win32_string(WINDOW_NAME).as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                null_mut(),
                null_mut(),
                window_class.hInstance,
                null_mut(),
            );

            if !window.is_null() {
                let mut x_offset = 0;
                let mut y_offset = 0;

                while RUNNING {
                    let mut message = zeroed();
                    while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                        if message.message == WM_QUIT {
                            RUNNING = false;
                        }

                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }

                    render_weird_gradient(x_offset, y_offset);

                    let device_context = GetDC(window);
                    let mut client_rect = windef::RECT::default();
                    GetClientRect(window, &mut client_rect);
                    update_window(device_context, &client_rect);
                    ReleaseDC(window, device_context);

                    x_offset += 1;
                    y_offset += 2;
                }
            } else {
                // TODO: logging
            }
        } else {
            // TODO: logging
        }
    }
}
