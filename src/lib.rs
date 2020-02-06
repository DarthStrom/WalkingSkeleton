//! equivalent to handmade.h & handmade.cpp

pub mod common;
use common::*;

#[macro_use]
extern crate log;

use std::f32::{self, consts::PI};

struct CanonicalPosition {
    /* TODO:

        Take the tile map x and y
        and the tile x and y

        and pack them into single 32-bit values for x and y
        where there is some low bits for the tile index
        and the high bits are the tile "page"

        (NOTE we can eliminate the need for floor!)
    */
    tile_map_x: i32,
    tile_map_y: i32,

    tile_x: i32,
    tile_y: i32,

    /* TODO:

        Convert these to math-friendly, resolution independent representation of
        world units relative to a tile.

    */
    tile_rel_x: f32,
    tile_rel_y: f32,
}

#[derive(Clone)]
struct RawPosition {
    tile_map_x: i32,
    tile_map_y: i32,

    // Tile-map relative x and y
    x: f32,
    y: f32,
}

struct TileMap<'a> {
    tiles: &'a [[u32; 17]; 9],
}

struct World<'a> {
    tile_side_in_meters: f32,
    tile_side_in_pixels: i32,

    count_x: i32,
    count_y: i32,

    upper_left_x: f32,
    upper_left_y: f32,

    // TODO: Beginner's sparseness
    tile_map_count_x: i32,
    tile_map_count_y: i32,

    tile_maps: &'a [[TileMap<'a>; 2]; 2],
}

struct State {
    // TODO: Player state should be canonical position now?
    player_tile_map_x: i32,
    player_tile_map_y: i32,

    player_x: f32,
    player_y: f32,
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

const TILE_MAPS: [[TileMap; 2]; 2] = [
    [TileMap { tiles: &TILES_00 }, TileMap { tiles: &TILES_10 }],
    [TileMap { tiles: &TILES_01 }, TileMap { tiles: &TILES_11 }],
];

const TILE_SIDE_IN_PIXELS: i32 = 60;
const WORLD: World = World {
    tile_map_count_x: 2,
    tile_map_count_y: 2,
    count_x: TILE_MAP_COUNT_X,
    count_y: TILE_MAP_COUNT_Y,
    upper_left_x: (-TILE_SIDE_IN_PIXELS / 2) as f32,
    upper_left_y: 0.0,
    tile_side_in_meters: 1.4,
    tile_side_in_pixels: TILE_SIDE_IN_PIXELS,
    tile_maps: &TILE_MAPS,
};

const PLAYER_WIDTH: f32 = 0.75 * WORLD.tile_side_in_pixels as f32;
const PLAYER_HEIGHT: f32 = WORLD.tile_side_in_pixels as f32;

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
        (*game_state).player_x = 175.0;
        (*game_state).player_y = 150.0;

        (*memory).is_initialized = true;
    }

    let tile_map = get_tile_map(
        &WORLD,
        (*game_state).player_tile_map_x,
        (*game_state).player_tile_map_y,
    )
    .expect("could not get tile map");

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

            let player_pos = RawPosition {
                tile_map_x: (*game_state).player_tile_map_x,
                tile_map_y: (*game_state).player_tile_map_y,
                x: new_player_x,
                y: new_player_y,
            };
            let mut player_left = player_pos.clone();
            player_left.x -= 0.5 * PLAYER_WIDTH;
            let mut player_right = player_pos.clone();
            player_right.x += 0.5 * PLAYER_WIDTH;

            if is_world_point_empty(&WORLD, &player_pos)
                && is_world_point_empty(&WORLD, &player_left)
                && is_world_point_empty(&WORLD, &player_right)
            {
                let can_pos = get_canonical_position(&WORLD, &player_pos);

                (*game_state).player_tile_map_x = can_pos.tile_map_x;
                (*game_state).player_tile_map_y = can_pos.tile_map_y;
                (*game_state).player_x = WORLD.upper_left_x
                    + WORLD.tile_side_in_pixels as f32 * can_pos.tile_x as f32
                    + can_pos.tile_rel_x;
                (*game_state).player_y = WORLD.upper_left_y
                    + WORLD.tile_side_in_pixels as f32 * can_pos.tile_y as f32
                    + can_pos.tile_rel_y;
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
            let tile_id = get_tile_value_unchecked(&WORLD, &tile_map, column, row);
            let gray = if tile_id == 1 { 1.0 } else { 0.5 };

            let min_x = WORLD.upper_left_x + column as f32 * WORLD.tile_side_in_pixels as f32;
            let min_y = WORLD.upper_left_y + row as f32 * WORLD.tile_side_in_pixels as f32;
            let max_x = min_x + WORLD.tile_side_in_pixels as f32;
            let max_y = min_y + WORLD.tile_side_in_pixels as f32;
            draw_rectangle(&(*buffer), min_x, min_y, max_x, max_y, gray, gray, gray);
        }
    }

    let player_r = 1.0;
    let player_g = 1.0;
    let player_b = 0.0;
    let player_left = (*game_state).player_x - 0.5 * PLAYER_WIDTH;
    let player_top = (*game_state).player_y - PLAYER_HEIGHT;
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

fn get_tile_value_unchecked(world: &World, tile_map: &TileMap, tile_x: i32, tile_y: i32) -> u32 {
    debug_assert!(tile_x >= 0 && tile_x < world.count_x && tile_y >= 0 && tile_y < world.count_y);

    tile_map.tiles[tile_y as usize][tile_x as usize]
}

fn is_tile_map_point_empty(
    world: &World,
    tile_map: &TileMap,
    test_tile_x: i32,
    test_tile_y: i32,
) -> bool {
    if test_tile_x >= 0
        && test_tile_x < world.count_x
        && test_tile_y >= 0
        && test_tile_y < world.count_y
    {
        get_tile_value_unchecked(world, tile_map, test_tile_x, test_tile_y) == 0
    } else {
        false
    }
}

fn get_canonical_position(world: &World, pos: &RawPosition) -> CanonicalPosition {
    let x = pos.x - world.upper_left_x;
    let y = pos.y - world.upper_left_y;
    let tile_x = (x / world.tile_side_in_pixels as f32).floor() as i32;
    let tile_y = (y / world.tile_side_in_pixels as f32).floor() as i32;
    let mut result = CanonicalPosition {
        tile_map_x: pos.tile_map_x,
        tile_map_y: pos.tile_map_y,
        tile_x,
        tile_y,
        tile_rel_x: x - tile_x as f32 * world.tile_side_in_pixels as f32,
        tile_rel_y: y - tile_y as f32 * world.tile_side_in_pixels as f32,
    };

    debug_assert!(result.tile_rel_x >= 0.0);
    debug_assert!(result.tile_rel_y >= 0.0);
    debug_assert!(result.tile_rel_x < world.tile_side_in_pixels as f32);
    debug_assert!(result.tile_rel_y < world.tile_side_in_pixels as f32);

    if result.tile_x < 0 {
        result.tile_x += world.count_x;
        result.tile_map_x -= 1;
    }

    if result.tile_y < 0 {
        result.tile_y += world.count_y;
        result.tile_map_y -= 1;
    }

    if result.tile_x >= world.count_x {
        result.tile_x -= world.count_x;
        result.tile_map_x += 1;
    }

    if result.tile_y >= world.count_y {
        result.tile_y -= world.count_y;
        result.tile_map_y += 1;
    }

    result
}

fn is_world_point_empty(world: &World, test_pos: &RawPosition) -> bool {
    let can_pos = get_canonical_position(world, test_pos);
    if let Some(tile_map) = get_tile_map(world, can_pos.tile_map_x, can_pos.tile_map_y) {
        is_tile_map_point_empty(world, tile_map, can_pos.tile_x, can_pos.tile_y)
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
