//! equivalent to handmade_platform.cpp

use core::mem::*;
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

unsafe fn get_alignment_offset(arena: *mut MemoryArena, alignment: usize) -> usize {
    let mut alignment_offset = 0;
    let result_pointer = (*arena).base as usize + (*arena).used;
    let alignment_mask = alignment.saturating_sub(1);
    if (result_pointer & alignment_mask) > 0 {
        alignment_offset = alignment - (result_pointer & alignment_mask);
    }
    alignment_offset
}

/// Uses PushSize for the correct amount and returns the pointer already cast to
/// the correct type for you.
pub unsafe fn push_struct<T>(arena: *mut MemoryArena) -> *mut T {
    push_size(arena, size_of::<T>(), Some(align_of::<T>())) as *mut T
}

/// Pushes the given number of bytes into the arena. Panics on OOM.
pub unsafe fn push_size(
    arena: *mut MemoryArena,
    size_init: usize,
    alignment: Option<usize>,
) -> *mut u8 {
    let alignment = alignment.unwrap_or(4);
    let mut size = size_init;

    let alignment_offset = get_alignment_offset(arena, alignment);
    size += alignment_offset;

    debug_assert!((*arena).used + size <= (*arena).size);

    let result = (*arena)
        .base
        .offset((*arena).used as isize + alignment_offset as isize);
    (*arena).used += size;

    debug_assert!(size >= size_init);

    result
}

/// This pushes the size of the type specified times the count specified.
///
/// Note that you can use `PushStruct` to push a Rust array of whatever size
/// (eg: `[u16; 20]`), which will have a similar effect, but this allows you to
/// select a size to push at runtime, which cannot currently be done with
/// PushStruct because Rust arrays must have a known size at compile time.
pub unsafe fn push_array<T>(arena: *mut MemoryArena, count: usize) -> *mut T {
    push_size(arena, size_of::<T>() * count, Some(align_of::<T>())) as *mut T
}

pub struct MemoryArena {
    pub size: usize,
    pub base: *mut u8,
    pub used: usize,
}

pub struct GameOffscreenBuffer {
    pub memory: *mut c_void,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub bytes_per_pixel: i32,
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
    pub dt_for_frame: f32,
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

pub unsafe fn get_controller(
    input: *mut GameInput,
    controller_index: usize,
) -> *mut GameControllerInput {
    debug_assert!(controller_index < (*input).controllers.len());
    &mut (*input).controllers[controller_index]
}
