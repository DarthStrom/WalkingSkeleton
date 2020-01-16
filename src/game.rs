// use super::platform;
use winapi::ctypes::c_void;

// TODO: services that the platform layer provides to the game

// services that the game provides to the platform layer
pub fn game_update_and_render(buffer: &OffscreenBuffer, blue_offset: i32, green_offset: i32) {
    unsafe {
        render_weird_gradient(buffer, blue_offset, green_offset);
    }
}

pub struct OffscreenBuffer {
    pub memory: *mut c_void,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub bytes_per_pixel: i32,
}

unsafe fn render_weird_gradient(buffer: &OffscreenBuffer, blue_offset: i32, green_offset: i32) {
    for y in 0..buffer.height {
        let row = (buffer.memory as *mut u8).offset((y * buffer.pitch) as isize);
        for x in 0..buffer.width {
            let pixel = row.offset((x * buffer.bytes_per_pixel) as isize);
            let blue = pixel;
            let green = pixel.offset(1);
            let red = pixel.offset(2);
            *red = 0;
            *green = (y + green_offset) as u8;
            *blue = (x + blue_offset) as u8;
        }
    }
}
