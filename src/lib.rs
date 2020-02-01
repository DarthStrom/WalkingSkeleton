//! equivalent to handmade.h & handmade.cpp

pub mod common;
use common::*;

#[macro_use]
extern crate log;

use std::f32::{self, consts::PI};

struct State {
    tone_hz: u32,
    green_offset: i32,
    blue_offset: i32,
    t_sine: f32,
    player_x: i32,
    player_y: i32,
    t_jump: f32,
}

/// This ensures that GameUpdateAndRender has a signature that will match what
/// is specified in handmade_platform.rs
const _UPDATE_CHECK: GameUpdateAndRender = update_and_render;

#[no_mangle]
pub unsafe extern "C" fn update_and_render(
    memory: *mut GameMemory,
    input: *mut GameInput,
    buffer: *mut GameOffscreenBuffer,
) {
    debug_assert!(std::mem::size_of::<State>() <= (*memory).permanent_storage_size);

    #[allow(clippy::cast_ptr_alignment)]
    let game_state = (*memory).permanent_storage as *mut State;

    if !(*memory).is_initialized {
        let contents = std::fs::read_to_string(file!()).expect("could not read file");
        std::fs::write("test.out", contents).expect("could not write file");

        (*game_state).tone_hz = 512;
        (*game_state).t_sine = 0.0;

        (*game_state).player_x = 100;
        (*game_state).player_y = 100;

        (*memory).is_initialized = true;
    }

    for controller_index in 0..(*input).controllers.len() {
        let controller = common::get_controller(input, controller_index);
        if (*controller).is_analog {
            trace!("use analog movement tuning");
            (*game_state).blue_offset += (4.0 * (*controller).stick_average_x) as i32;
            (*game_state).tone_hz = (512.0 + 128.0 * (*controller).stick_average_y) as u32;
        } else {
            trace!("use digital movement tuning");

            if (*controller).move_left.ended_down {
                (*game_state).blue_offset -= 1;
            }

            if (*controller).move_right.ended_down {
                (*game_state).blue_offset += 1
            }
        }

        (*game_state).player_x += (4.0 * (*controller).stick_average_x) as i32;
        (*game_state).player_y += (4.0 * (*controller).stick_average_y) as i32;
        if (*game_state).t_jump > 0.0 {
            (*game_state).player_y += (5.0 * (0.5 * PI * (*game_state).t_jump).sin()) as i32;
        }

        if (*controller).action_down.ended_down {
            (*game_state).t_jump = 4.0;
        }
        (*game_state).t_jump -= 0.033;
    }

    render_weird_gradient(
        &(*buffer),
        (*game_state).blue_offset,
        (*game_state).green_offset,
    );
    render_player(&(*buffer), (*game_state).player_x, (*game_state).player_y);
}

/// This ensures that GameGetSoundSamples has a signature that will match what
/// is specified in handmade_platform.rs
const _SOUND_CHECK: GameGetSoundSamples = get_sound_samples;

// At the moment, this has to be a very fast function, it cannot be
// more than a millisecond or so.
// TODO: Reduce the pressure on this function's performance by measuring it
// or asking about it, etc.
#[no_mangle]
pub unsafe extern "C" fn get_sound_samples(
    memory: *mut GameMemory,
    sound_buffer: *mut GameSoundOutputBuffer,
) {
    // TODO: understand this and see if we can remove the clippy exception
    #[allow(clippy::cast_ptr_alignment)]
    let game_state = (*memory).permanent_storage as *mut State;
    output_sound(&mut (*game_state), sound_buffer, (*game_state).tone_hz);
}

// TODO: platform independent code should get priority for removing unsafe
unsafe fn output_sound(
    game_state: &mut State,
    sound_buffer: *mut GameSoundOutputBuffer,
    tone_hz: u32,
) {
    let tone_volume = 3_000.0;
    let wave_period = (*sound_buffer).samples_per_second / tone_hz;

    let mut sample_out = (*sound_buffer).samples;
    for _ in 0..(*sound_buffer).sample_count {
        let sine_value = game_state.t_sine.sin();
        let sample_value = (sine_value * tone_volume) as i16;
        // let sample_value = 0;

        *sample_out = sample_value;
        sample_out = sample_out.offset(1);
        *sample_out = sample_value;
        sample_out = sample_out.offset(1);

        let tau = 2.0 * PI;
        game_state.t_sine += tau / wave_period as f32;
        if game_state.t_sine > tau {
            game_state.t_sine -= tau;
        }
    }
}

unsafe fn render_weird_gradient(buffer: &GameOffscreenBuffer, blue_offset: i32, green_offset: i32) {
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

unsafe fn render_player(buffer: &GameOffscreenBuffer, player_x: i32, player_y: i32) {
    let end_of_buffer = (buffer.memory as *mut u8).offset((buffer.pitch * buffer.height) as isize);

    let color: u32 = 0xFF_FF_FF_FF;
    let top = player_y;
    let bottom = player_y + 10;
    for x in player_x..player_x + 10 {
        let mut pixel = (buffer.memory as *mut u8)
            .offset((x * buffer.bytes_per_pixel) as isize)
            .offset((top * buffer.pitch) as isize);

        // TODO: fix this
        #[allow(clippy::cast_ptr_alignment)]
        for _y in top..bottom {
            if pixel >= buffer.memory as *mut u8 && (pixel.offset(4)) <= end_of_buffer {
                *(pixel as *mut u32) = color;
            }

            pixel = pixel.offset(buffer.pitch as isize);
        }
    }
}
