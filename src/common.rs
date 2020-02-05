//! equivalent to handmade_platform.cpp

use winapi::ctypes::c_void;

pub fn kilobytes(bytes: usize) -> usize {
    bytes * 1024
}

pub fn megabytes(bytes: usize) -> usize {
    1024 * kilobytes(bytes)
}

pub fn gigabytes(bytes: usize) -> usize {
    1024 * megabytes(bytes)
}

pub fn terabytes(bytes: usize) -> usize {
    1024 * gigabytes(bytes)
}

pub struct GameOffscreenBuffer {
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

#[derive(Debug, Default)]
pub struct GameButtonState {
    pub half_transition_count: i32,
    pub ended_down: bool,
}

#[derive(Default)]
pub struct GameControllerInput {
    pub is_connected: bool,
    pub is_analog: bool,
    pub stick_average_x: f32,
    pub stick_average_y: f32,

    pub move_up: GameButtonState,
    pub move_down: GameButtonState,
    pub move_left: GameButtonState,
    pub move_right: GameButtonState,

    pub action_up: GameButtonState,
    pub action_down: GameButtonState,
    pub action_left: GameButtonState,
    pub action_right: GameButtonState,

    pub left_shoulder: GameButtonState,
    pub right_shoulder: GameButtonState,

    pub select: GameButtonState,
    pub start: GameButtonState,

    pub terminator: GameButtonState,
}

pub struct GameInput {
    pub mouse_buttons: [GameButtonState; 5],
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_z: i32,
    pub seconds_to_advance_over_update: f32,
    pub controllers: [GameControllerInput; 5],
}

#[derive(Debug)]
pub struct GameMemory {
    pub is_initialized: bool,
    pub permanent_storage_size: usize,
    // required to be cleared to zero at startup
    pub permanent_storage: *mut u8,
    pub transient_storage_size: usize,
    // required to be cleared to zero at startup
    pub transient_storage: *mut u8,
}

pub type GameUpdateAndRender =
    unsafe extern "C" fn(*mut GameMemory, *mut GameInput, *mut GameOffscreenBuffer);
pub type GameGetSoundSamples = unsafe extern "C" fn(*mut GameMemory, *mut GameSoundOutputBuffer);

pub unsafe fn get_controller(
    input: *mut GameInput,
    controller_index: usize,
) -> *mut GameControllerInput {
    debug_assert!(controller_index < (*input).controllers.len());
    &mut (*input).controllers[controller_index]
}
