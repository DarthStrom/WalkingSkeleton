//! equivalent to handmade.h & handmade.cpp

pub mod common;
use common::*;

#[macro_use]
extern crate log;

use std::f32::{self, consts::PI};

struct State {
    player_x: f32,
    player_y: f32,
}

struct TileMap<'a> {
    count_x: i32,
    count_y: i32,

    upper_left_x: f32,
    upper_left_y: f32,
    tile_width: f32,
    tile_height: f32,

    tiles: &'a [[u32; 17]; 9],
}

struct World<'a> {
    // TODO: Beginner's sparseness
    tile_map_count_x: i32,
    tile_map_count_y: i32,

    tile_maps: &'a [[TileMap<'a>; 2]; 2],
}

/// This ensures that GameUpdateAndRender has a signature that will match what
/// is specified in handmade_platform.rs
const _UPDATE_CHECK: GameUpdateAndRender = update_and_render;

const TILE_MAP_COUNT_X: i32 = 17;
const TILE_MAP_COUNT_Y: i32 = 9;
const TILES_00: [[u32; 17]; 9] = [
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
    [1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
];

const TILES_01: [[u32; 17]; 9] = [
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
];

const TILES_10: [[u32; 17]; 9] = [
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
];

const TILES_11: [[u32; 17]; 9] = [
    [1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
];

const TILE_WIDTH: f32 = 60.0;
const TILE_HEIGHT: f32 = 60.0;

const TILE_MAP_00: TileMap = TileMap {
    count_x: TILE_MAP_COUNT_X,
    count_y: TILE_MAP_COUNT_Y,
    upper_left_x: -30.0,
    upper_left_y: 0.0,
    tile_width: TILE_WIDTH,
    tile_height: TILE_HEIGHT,
    tiles: &TILES_00,
};
const TILE_MAPS: [[TileMap; 2]; 2] = [
    [
        TILE_MAP_00,
        TileMap {
            tiles: &TILES_01,
            ..TILE_MAP_00
        },
    ],
    [
        TileMap {
            tiles: &TILES_10,
            ..TILE_MAP_00
        },
        TileMap {
            tiles: &TILES_11,
            ..TILE_MAP_00
        },
    ],
];

const WORLD: World = World {
    tile_map_count_x: 2,
    tile_map_count_y: 2,
    tile_maps: &TILE_MAPS,
};

const PLAYER_WIDTH: f32 = 0.75 * TILE_WIDTH;
const PLAYER_HEIGHT: f32 = TILE_HEIGHT;

static mut TILE_MAP: &TileMap = &TILE_MAPS[0][0];

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
        (*game_state).player_x = 150.0;
        (*game_state).player_y = 150.0;

        (*memory).is_initialized = true;
    }

    for controller_index in 0..(*input).controllers.len() {
        let controller = common::get_controller(input, controller_index);
        if (*controller).is_analog {
            trace!("use analog movement tuning");
        } else {
            trace!("use digital movement tuning");
            let d_player_y = 64.0
                * if (*controller).move_up.ended_down {
                    -1.0
                } else if (*controller).move_down.ended_down {
                    1.0
                } else {
                    0.0
                };
            let d_player_x = 64.0
                * if (*controller).move_left.ended_down {
                    -1.0
                } else if (*controller).move_right.ended_down {
                    1.0
                } else {
                    0.0
                };

            // TODO: Diagonal will be faster! Fix once we have vectors
            let new_player_x = (*game_state).player_x + (*input).dt_for_frame * d_player_x;
            let new_player_y = (*game_state).player_y + (*input).dt_for_frame * d_player_y;

            if is_tile_map_point_empty(&TILE_MAP, new_player_x - 0.5 * PLAYER_WIDTH, new_player_y)
                && is_tile_map_point_empty(
                    &TILE_MAP,
                    new_player_x + 0.5 * PLAYER_WIDTH,
                    new_player_y,
                )
                && is_tile_map_point_empty(&TILE_MAP, new_player_x, new_player_y)
            {
                (*game_state).player_x = new_player_x;
                (*game_state).player_y = new_player_y;
            }
        }
    }

    draw_rectangle(
        &(*buffer),
        0.0,
        0.0,
        (*buffer).width as f32,
        (*buffer).height as f32,
        1.0,
        0.0,
        0.1,
    );
    for row in 0..9 {
        for column in 0..17 {
            let tile_id = get_tile_value_unchecked(&TILE_MAP, column, row);
            let gray = if tile_id == 1 { 1.0 } else { 0.5 };

            let min_x = TILE_MAP.upper_left_x + column as f32 * TILE_MAP.tile_width;
            let min_y = TILE_MAP.upper_left_y + row as f32 * TILE_MAP.tile_height;
            let max_x = min_x + TILE_MAP.tile_width;
            let max_y = min_y + TILE_MAP.tile_height;
            draw_rectangle(&(*buffer), min_x, min_y, max_x, max_y, gray, gray, gray);
        }
    }

    let player_r = 1.0;
    let player_g = 1.0;
    let player_b = 0.0;
    let player_left = (*game_state).player_x - 0.5 * PLAYER_WIDTH;
    let player_top = (*game_state).player_y - PLAYER_HEIGHT;
    println!("player: {}", player_left);
    draw_rectangle(
        &(*buffer),
        player_left,
        player_top,
        player_left + PLAYER_WIDTH,
        player_top + PLAYER_HEIGHT,
        player_r,
        player_g,
        player_b,
    );
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
    output_sound(&mut (*game_state), sound_buffer, 400);
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
        // let sine_value = game_state.t_sine.sin();
        // let sample_value = (sine_value * tone_volume) as i16;
        let sample_value = 0;

        *sample_out = sample_value;
        sample_out = sample_out.offset(1);
        *sample_out = sample_value;
        sample_out = sample_out.offset(1);

        // let tau = 2.0 * PI;
        // game_state.t_sine += tau / wave_period as f32;
        // if game_state.t_sine > tau {
        //     game_state.t_sine -= tau;
        // }
    }
}

fn draw_rectangle(
    buffer: &GameOffscreenBuffer,
    real_min_x: f32,
    real_min_y: f32,
    real_max_x: f32,
    real_max_y: f32,
    r: f32,
    g: f32,
    b: f32,
) {
    // TODO: Floating point color

    let mut min_x = real_min_x.round() as i32;
    let mut min_y = real_min_y.round() as i32;
    let mut max_x = real_max_x.round() as i32;
    let mut max_y = real_max_y.round() as i32;

    if min_x < 0 {
        min_x = 0
    };

    if min_y < 0 {
        min_y = 0
    };

    if max_x > buffer.width {
        max_x = buffer.width
    };

    if max_y > buffer.height {
        max_y = buffer.height
    };

    let color = ((r * 255.0).round() as u32) << 16
        | ((g * 255.0).round() as u32) << 8
        | (b * 255.0).round() as u32;

    unsafe {
        let mut row = (buffer.memory as *mut u8)
            .offset((min_x * buffer.bytes_per_pixel) as isize)
            .offset((min_y * buffer.pitch) as isize);
        for _y in min_y..max_y {
            #[allow(clippy::cast_ptr_alignment)]
            let mut pixel = row as *mut u32;
            for _x in min_x..max_x {
                *pixel = color;
                pixel = pixel.offset(1);
            }
            row = row.offset(buffer.pitch as isize);
        }
    }
}

fn get_tile_map<'a>(world: &'a World, tile_map_x: i32, tile_map_y: i32) -> Option<&'a TileMap<'a>> {
    if tile_map_x >= 0
        && tile_map_x < world.tile_map_count_x
        && tile_map_y >= 0
        && tile_map_y < world.tile_map_count_y
    {
        Some(&world.tile_maps[tile_map_y as usize][tile_map_x as usize])
    } else {
        None
    }
}

fn get_tile_value_unchecked(tile_map: &TileMap, tile_x: i32, tile_y: i32) -> u32 {
    tile_map.tiles[tile_y as usize][tile_x as usize]
}

fn is_tile_map_point_empty(tile_map: &TileMap, test_x: f32, test_y: f32) -> bool {
    let player_tile_x = ((test_x - tile_map.upper_left_x) / tile_map.tile_width) as i32;
    let player_tile_y = ((test_y - tile_map.upper_left_y) / tile_map.tile_height) as i32;

    if player_tile_x >= 0
        && player_tile_x < tile_map.count_x
        && player_tile_y >= 0
        && player_tile_y < tile_map.count_y
    {
        get_tile_value_unchecked(tile_map, player_tile_x, player_tile_y) == 0
    } else {
        false
    }
}

fn is_world_point_empty(
    world: &World,
    tile_map_x: i32,
    tile_map_y: i32,
    test_x: f32,
    test_y: f32,
) -> bool {
    if let Some(tile_map) = get_tile_map(&world, tile_map_x, tile_map_y) {
        let player_tile_x = ((test_x - tile_map.upper_left_x) / tile_map.tile_width) as i32;
        let player_tile_y = ((test_y - tile_map.upper_left_y) / tile_map.tile_height) as i32;

        if player_tile_x >= 0
            && player_tile_x < tile_map.count_x
            && player_tile_y >= 0
            && player_tile_y < tile_map.count_y
        {
            get_tile_value_unchecked(tile_map, player_tile_x, player_tile_y) == 0
        } else {
            false
        }
    } else {
        false
    }
}

// unsafe fn render_weird_gradient(buffer: &GameOffscreenBuffer, blue_offset: i32, green_offset: i32) {
//     for y in 0..buffer.height {
//         let row = (buffer.memory as *mut u8).offset((y * buffer.pitch) as isize);
//         for x in 0..buffer.width {
//             let pixel = row.offset((x * buffer.bytes_per_pixel) as isize);
//             let blue = pixel;
//             let green = pixel.offset(1);
//             let red = pixel.offset(2);
//             *red = 0;
//             *green = (y + green_offset) as u8;
//             *blue = (x + blue_offset) as u8;
//         }
//     }
// }
