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

pub struct SoundOutputBuffer {
    pub samples_per_second: u32,
    pub sample_count: u32,
    // IMPORTANT: samples must be padded to a multiple of 4
    pub samples: *mut i16,
}

#[derive(Debug, Default)]
pub struct ButtonState {
    pub half_transition_count: i32,
    pub ended_down: bool,
}

#[derive(Default)]
pub struct ControllerInput {
    pub is_analog: bool,
    pub start_x: f32,
    pub start_y: f32,
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub up: ButtonState,
    pub down: ButtonState,
    pub left: ButtonState,
    pub right: ButtonState,
    pub left_shoulder: ButtonState,
    pub right_shoulder: ButtonState,
}

pub struct Input {
    pub controllers: [ControllerInput; 4],
}

#[derive(Debug)]
pub struct Memory {
    pub is_initialized: bool,
    pub permanent_storage_size: usize,
    // required to be cleared to zero at startup
    pub permanent_storage: *mut u8,
    pub transient_storage_size: usize,
    // required to be cleared to zero at startup
    pub transient_storage: *mut u8,
}

pub struct State {
    pub tone_hz: u32,
    pub green_offset: i32,
    pub blue_offset: i32,
}

// services that the game provides to the platform layer
pub fn update_and_render(
    memory: &mut Memory,
    input: &Input,
    buffer: &OffscreenBuffer,
    sound_buffer: &SoundOutputBuffer,
) {
    debug_assert!(std::mem::size_of::<State>() <= memory.permanent_storage_size);

    unsafe {
        let game_state = memory.permanent_storage as *mut State;

        if !memory.is_initialized {
            let contents = std::fs::read_to_string(file!()).expect("could not read file");
            std::fs::write("test.out", contents).expect("could not write file");

            (*game_state).tone_hz = 256;
            memory.is_initialized = true;
        }

        let input0 = &input.controllers[0];
        if input0.is_analog {
            // use analog movement tuning
            (*game_state).blue_offset += (4.0 * input0.end_x) as i32;
            (*game_state).tone_hz = (256.0 + 128.0 * input0.end_y) as u32;
        } else {
            // use digital movement tuning
        }

        if input0.down.ended_down {
            (*game_state).green_offset += 1;
        }

        output_sound(sound_buffer, (*game_state).tone_hz);
        render_weird_gradient(
            buffer,
            (*game_state).blue_offset,
            (*game_state).green_offset,
        )
    };
}

// TODO: platform independent code should get priority for removing unsafe
unsafe fn output_sound(sound_buffer: &SoundOutputBuffer, tone_hz: u32) {
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
