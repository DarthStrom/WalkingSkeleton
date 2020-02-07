//! equivalent to handmade.h & handmade.cpp

pub mod common;
use common::*;

#[macro_use]
extern crate log;

use std::f32::{self, consts::PI};

type Tiles = [[u32; TILE_MAP_COUNT_X]; TILE_MAP_COUNT_Y];

#[derive(Clone)]
struct TileChunkPosition {
    tile_chunk_x: u32,
    tile_chunk_y: u32,

    rel_tile_x: u32,
    rel_tile_y: u32,
}

#[derive(Clone)]
struct WorldPosition {
    /* TODO:

        Take the tile map x and y
        and the tile x and y

        and pack them into single 32-bit values for x and y
        where there is some low bits for the tile index
        and the high bits are the tile "page"

        (NOTE we can eliminate the need for floor!)
    */
    abs_tile_x: u32,
    abs_tile_y: u32,

    // TODO: Should these be from the center of a tile?
    // TODO: Rename to offset x and y
    tile_rel_x: f32,
    tile_rel_y: f32,
}

struct TileChunk<'a> {
    tiles: &'a Tiles,
}

struct World<'a> {
    chunk_shift: i32,
    chunk_mask: u32,
    chunk_dim: u32,

    tile_side_in_meters: f32,
    tile_side_in_pixels: i32,
    meters_to_pixels: f32,

    // TODO: Beginner's sparseness
    tile_chunk_count_x: u32,
    tile_chunk_count_y: u32,

    tile_chunks: Vec<TileChunk<'a>>,
}

struct State {
    player_p: WorldPosition,
}

/// This ensures that GameUpdateAndRender has a signature that will match what
/// is specified in handmade_platform.rs
const _UPDATE_CHECK: GameUpdateAndRender = update_and_render;

const TILE_MAP_COUNT_X: usize = 256;
const TILE_MAP_COUNT_Y: usize = 256;
const TEMP_TILES: [[u32; 34]; 18] = [
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1,
    ],
    [
        1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1,
        1, 1, 1, 1,
    ],
    [
        1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1,
        1, 1, 1, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1,
    ],
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1,
    ],
];

const TILE_SIDE_IN_PIXELS: i32 = 60;
const TILE_SIDE_IN_METERS: f32 = 1.4;

const PLAYER_HEIGHT: f32 = 1.4;
const PLAYER_WIDTH: f32 = 0.75 * PLAYER_HEIGHT;

#[no_mangle]
pub unsafe extern "C" fn update_and_render(
    memory: *mut GameMemory,
    input: *mut GameInput,
    buffer: *mut GameOffscreenBuffer,
) {
    debug_assert!(std::mem::size_of::<State>() <= (*memory).permanent_storage_size);

    let tiles: &mut Tiles = &mut [[0; 256]; 256];
    for (row_index, row) in TEMP_TILES.iter().enumerate() {
        for (column_index, &column) in row.iter().enumerate() {
            tiles[row_index][column_index] = column
        }
    }
    let tile_chunk = TileChunk { tiles };
    let chunk_shift = 8;
    let world = World {
        chunk_shift,
        chunk_mask: (1 << chunk_shift) - 1,
        chunk_dim: 256,
        tile_chunk_count_x: 1,
        tile_chunk_count_y: 1,
        tile_chunks: vec![tile_chunk],
        tile_side_in_meters: TILE_SIDE_IN_METERS,
        tile_side_in_pixels: TILE_SIDE_IN_PIXELS,
        meters_to_pixels: TILE_SIDE_IN_PIXELS as f32 / TILE_SIDE_IN_METERS,
    };

    let lower_left_x = -(world.tile_side_in_pixels as f32 / 2.0);
    let lower_left_y = (*buffer).height as f32;

    #[allow(clippy::cast_ptr_alignment)]
    let game_state = (*memory).permanent_storage as *mut State;

    if !(*memory).is_initialized {
        (*game_state).player_p.abs_tile_x = 3;
        (*game_state).player_p.abs_tile_y = 3;
        (*game_state).player_p.tile_rel_x = 5.0;
        (*game_state).player_p.tile_rel_y = 5.0;

        (*memory).is_initialized = true;
    }

    for controller_index in 0..(*input).controllers.len() {
        let controller = common::get_controller(input, controller_index);
        if (*controller).is_analog {
            trace!("use analog movement tuning");
        } else {
            trace!("use digital movement tuning");
            let d_player_y = 2.0
                * if (*controller).move_up.ended_down {
                    1.0
                } else if (*controller).move_down.ended_down {
                    -1.0
                } else {
                    0.0
                };
            let d_player_x = 2.0
                * if (*controller).move_left.ended_down {
                    -1.0
                } else if (*controller).move_right.ended_down {
                    1.0
                } else {
                    0.0
                };

            // TODO: Diagonal will be faster! Fix once we have vectors
            let mut new_player_p = (*game_state).player_p.clone();
            new_player_p.tile_rel_x += (*input).dt_for_frame * d_player_x;
            new_player_p.tile_rel_y += (*input).dt_for_frame * d_player_y;
            new_player_p = recanonicalize_position(&world, new_player_p);
            // TODO: Delta function that auto-recanonicalizes

            let mut player_left = new_player_p.clone();
            player_left.tile_rel_x -= 0.5 * PLAYER_WIDTH;
            player_left = recanonicalize_position(&world, player_left);

            let mut player_right = new_player_p.clone();
            player_right.tile_rel_x += 0.5 * PLAYER_WIDTH;
            player_right = recanonicalize_position(&world, player_right);

            if is_world_point_empty(&world, new_player_p.clone())
                && is_world_point_empty(&world, player_left)
                && is_world_point_empty(&world, player_right)
            {
                (*game_state).player_p = new_player_p
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

    let center_x = 0.5 * (*buffer).width as f32;
    let center_y = 0.5 * (*buffer).height as f32;

    for r in 0..20 {
        for c in 0..40 {
            let rel_row = r - 10;
            let rel_column = c - 20;
            let column = ((*game_state).player_p.abs_tile_x as i32 + rel_column) as u32;
            let row = ((*game_state).player_p.abs_tile_y as i32 + rel_row) as u32;
            let tile_id = get_tile_value_abs(&world, column, row);
            let gray = if tile_id == 1 {
                1.0
            } else if column == (*game_state).player_p.abs_tile_x
                && row == (*game_state).player_p.abs_tile_y
            {
                0.0
            } else {
                0.5
            };

            let min_x = center_x + rel_column as f32 * world.tile_side_in_pixels as f32;
            let min_y = center_y - rel_row as f32 * world.tile_side_in_pixels as f32;
            let max_x = min_x + world.tile_side_in_pixels as f32;
            let max_y = min_y - world.tile_side_in_pixels as f32;
            draw_rectangle(&(*buffer), min_x, max_y, max_x, min_y, gray, gray, gray);
        }
    }

    let player_r = 1.0;
    let player_g = 1.0;
    let player_b = 0.0;
    let player_left = center_x + world.meters_to_pixels * (*game_state).player_p.tile_rel_x
        - 0.5 * world.meters_to_pixels * PLAYER_WIDTH;
    let player_top = center_y
        - world.meters_to_pixels * (*game_state).player_p.tile_rel_y
        - world.meters_to_pixels * PLAYER_HEIGHT;
    draw_rectangle(
        &(*buffer),
        player_left,
        player_top,
        player_left + world.meters_to_pixels * PLAYER_WIDTH,
        player_top + world.meters_to_pixels * PLAYER_HEIGHT,
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

fn get_tile_chunk<'a>(
    world: &'a World,
    tile_chunk_x: i32,
    tile_chunk_y: i32,
) -> Option<&'a TileChunk<'a>> {
    if tile_chunk_x >= 0
        && tile_chunk_x < world.tile_chunk_count_x as i32
        && tile_chunk_y >= 0
        && tile_chunk_y < world.tile_chunk_count_y as i32
    {
        Some(
            &world.tile_chunks
                [(tile_chunk_y * world.tile_chunk_count_x as i32 + tile_chunk_x) as usize],
        )
    } else {
        None
    }
}

fn get_tile_value(world: &World, tile_chunk: &TileChunk, tile_x: u32, tile_y: u32) -> u32 {
    debug_assert!(tile_x < world.chunk_dim);
    debug_assert!(tile_y < world.chunk_dim);

    tile_chunk.tiles[tile_y as usize][tile_x as usize]
}

fn recanonicalize_coord(world: &World, tile: &mut u32, tile_rel: &mut f32) {
    // TODO: Need to do something that doesn't use the divide/multiply method
    // for recanonicalizing because this can end up rounding back on to the tile
    // you just came from.

    // World is assumed to bo toroidal topology, if you
    // step off one end you come back on the other
    let offset = (*tile_rel / world.tile_side_in_meters).floor() as i32;
    *tile = (*tile as i32 + offset) as u32;
    *tile_rel -= offset as f32 * world.tile_side_in_meters;

    debug_assert!(*tile_rel >= 0.0);
    // TODO: Fix floating point math so this can be <
    debug_assert!(*tile_rel <= world.tile_side_in_meters);
}

fn recanonicalize_position(world: &World, pos: WorldPosition) -> WorldPosition {
    let mut result = pos;

    recanonicalize_coord(world, &mut result.abs_tile_x, &mut result.tile_rel_x);
    recanonicalize_coord(world, &mut result.abs_tile_y, &mut result.tile_rel_y);

    result
}

fn get_chunk_position_for(world: &World, abs_tile_x: u32, abs_tile_y: u32) -> TileChunkPosition {
    TileChunkPosition {
        tile_chunk_x: abs_tile_x >> world.chunk_shift,
        tile_chunk_y: abs_tile_y >> world.chunk_shift,
        rel_tile_x: abs_tile_x & world.chunk_mask,
        rel_tile_y: abs_tile_y & world.chunk_mask,
    }
}

fn get_tile_value_abs(world: &World, abs_tile_x: u32, abs_tile_y: u32) -> u32 {
    let chunk_pos = get_chunk_position_for(world, abs_tile_x, abs_tile_y);
    if let Some(tile_chunk) = get_tile_chunk(
        world,
        chunk_pos.tile_chunk_x as i32,
        chunk_pos.tile_chunk_y as i32,
    ) {
        get_tile_value(
            world,
            tile_chunk,
            chunk_pos.rel_tile_x,
            chunk_pos.rel_tile_y,
        )
    } else {
        0
    }
}

fn is_world_point_empty(world: &World, can_pos: WorldPosition) -> bool {
    get_tile_value_abs(world, can_pos.abs_tile_x, can_pos.abs_tile_y) == 0
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
