// use super::platform;
use std::f32::{self, consts::PI};
use winapi::ctypes::c_void;

static mut T_SINE: f32 = 0.0;

pub struct OffscreenBuffer {
    pub memory: *mut c_void,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub bytes_per_pixel: i32,
}

pub struct GameSoundOutputBuffer {
    pub samples_per_second: u32,
    pub sample_count: u32,
    // IMPORTANT: samples must be padded to a multiple of 4
    pub samples: *mut i16,
}

// TODO: services that the platform layer provides to the game

// services that the game provides to the platform layer
pub fn game_update_and_render(
    buffer: &OffscreenBuffer,
    blue_offset: i32,
    green_offset: i32,
    sound_buffer: &GameSoundOutputBuffer,
    tone_hz: u32,
) {
    unsafe {
        game_output_sound(sound_buffer, tone_hz);
        render_weird_gradient(buffer, blue_offset, green_offset)
    };
}

unsafe fn game_output_sound(sound_buffer: &GameSoundOutputBuffer, tone_hz: u32) {
    let tone_volume = 3_000.0;
    let wave_period = sound_buffer.samples_per_second / tone_hz;

    let mut sample_out = sound_buffer.samples;
    for _ in 0..sound_buffer.sample_count {
        let sine_value = (T_SINE).sin();
        let sample_value = (sine_value * tone_volume) as i16;

        *sample_out = sample_value;
        sample_out = sample_out.offset(1);
        *sample_out = sample_value;
        sample_out = sample_out.offset(1);

        let tau = 2.0 * PI;
        T_SINE += tau / wave_period as f32;
        if T_SINE > tau {
            T_SINE -= tau;
        }
    }
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
