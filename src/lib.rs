//! equivalent to handmade.h & handmade.cpp

pub mod common;
mod tile;

use common::*;
use core::mem::*;
use rand::prelude::*;
use tile::*;

#[macro_use]
extern crate log;

use std::f32::{self, consts::PI};

struct World {
    tile_map: *mut TileMap,
}

struct State {
    world_arena: MemoryArena,
    world: *mut World,

    player_p: TileMapPosition,
    pixel_pointer: *mut u32,
}

/// This ensures that GameUpdateAndRender has a signature that will match what
/// is specified in handmade_platform.rs
const _UPDATE_CHECK: GameUpdateAndRender = update_and_render;

const PLAYER_HEIGHT: f32 = 1.4;
const PLAYER_WIDTH: f32 = 0.75 * PLAYER_HEIGHT;

unsafe fn initialize_arena(arena: *mut MemoryArena, size: usize, base: *mut u8) {
    (*arena).size = size;
    (*arena).base = base;
    (*arena).used = 0;
}

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
        // let bmp = image::open("test/test_background.bmp").expect("could not load background");

        (*game_state).player_p.abs_tile_x = 1;
        (*game_state).player_p.abs_tile_y = 3;
        (*game_state).player_p.offset_x = 5.0;
        (*game_state).player_p.offset_y = 5.0;

        initialize_arena(
            &mut (*game_state).world_arena,
            (*memory).permanent_storage_size as usize - size_of::<State>(),
            ((*memory).permanent_storage as *mut u8).add(size_of::<State>()),
        );

        (*game_state).world = push_struct::<World>(&mut (*game_state).world_arena);
        let world = (*game_state).world;
        (*world).tile_map = push_struct::<TileMap>(&mut (*game_state).world_arena);

        let tile_map = (*world).tile_map;

        (*tile_map).chunk_shift = 4;
        (*tile_map).chunk_mask = (1 << (*tile_map).chunk_shift) - 1;
        (*tile_map).chunk_dim = 1 << (*tile_map).chunk_shift;

        (*tile_map).tile_chunk_count_x = 128;
        (*tile_map).tile_chunk_count_y = 128;
        (*tile_map).tile_chunk_count_z = 2;
        (*tile_map).tile_chunks = push_array::<TileChunk>(
            &mut (*game_state).world_arena,
            ((*tile_map).tile_chunk_count_x
                * (*tile_map).tile_chunk_count_y
                * (*tile_map).tile_chunk_count_z) as usize,
        );

        (*tile_map).tile_side_in_meters = 1.4;

        let tiles_per_width = 17;
        let tiles_per_height = 9;
        let mut screen_x = 0;
        let mut screen_y = 0;
        let mut abs_tile_z = 0;

        //TODO: Replace all this with real world generation
        let mut door_left = false;
        let mut door_right = false;
        let mut door_top = false;
        let mut door_bottom = false;
        let mut door_up = false;
        let mut door_down = false;
        for screen_index in 0..100 {
            let mut rng = thread_rng();
            let random_choice = if door_up || door_down {
                rng.gen_range(0, 2)
            } else {
                rng.gen_range(0, 3)
            };

            let mut created_z_door = false;
            match random_choice {
                2 => {
                    created_z_door = true;
                    if abs_tile_z == 0 {
                        door_up = true;
                    } else {
                        door_down = true;
                    }
                }
                1 => {
                    door_right = true;
                }
                _ => {
                    door_top = true;
                }
            }

            for tile_y in 0..tiles_per_height {
                for tile_x in 0..tiles_per_width {
                    let abs_tile_x = screen_x * tiles_per_width + tile_x;
                    let abs_tile_y = screen_y * tiles_per_height + tile_y;

                    let tile_value = if door_down && tile_x == 10 && tile_y == 6 {
                        4
                    } else if door_up && tile_x == 10 && tile_y == 6 {
                        3
                    } else if ((tile_x == 0) && (!door_left || (tile_y != (tiles_per_height / 2))))
                        || ((tile_x == (tiles_per_width - 1))
                            && (!door_right || (tile_y != (tiles_per_height / 2))))
                        || ((tile_y == 0) && (!door_bottom || (tile_x != (tiles_per_width / 2))))
                        || ((tile_y == (tiles_per_height - 1))
                            && (!door_top || (tile_x != (tiles_per_width / 2))))
                    {
                        2
                    } else {
                        1
                    };

                    set_tile_value(
                        &mut (*game_state).world_arena,
                        (*world).tile_map,
                        abs_tile_x,
                        abs_tile_y,
                        abs_tile_z,
                        tile_value,
                    );
                }
            }

            door_left = door_right;
            door_bottom = door_top;

            if created_z_door {
                door_down = !door_down;
                door_up = !door_up;
            } else {
                door_up = false;
                door_down = false;
            }

            door_right = false;
            door_top = false;

            match random_choice {
                2 => {
                    if abs_tile_z == 0 {
                        abs_tile_z = 1;
                    } else {
                        abs_tile_z = 0;
                    }
                }
                1 => screen_x += 1,
                _ => screen_y += 1,
            }
        }

        (*memory).is_initialized = true;
    }

    let world = (*game_state).world;
    let tile_map = (*world).tile_map;

    let tile_side_in_pixels = 60;
    let meters_to_pixels = tile_side_in_pixels as f32 / (*tile_map).tile_side_in_meters;

    let lower_left_x = -(tile_side_in_pixels as f32 / 2.0);
    let lower_left_y = (*buffer).height as f32;

    for controller_index in 0..(*input).controllers.len() {
        let controller = common::get_controller(input, controller_index);
        if (*controller).is_analog {
            trace!("use analog movement tuning");
        } else {
            trace!("use digital movement tuning");
            let player_speed = if (*controller).action_up.ended_down {
                10.0
            } else {
                2.0
            };
            let d_player_y = player_speed
                * if (*controller).move_up.ended_down {
                    1.0
                } else if (*controller).move_down.ended_down {
                    -1.0
                } else {
                    0.0
                };
            let d_player_x = player_speed
                * if (*controller).move_left.ended_down {
                    -1.0
                } else if (*controller).move_right.ended_down {
                    1.0
                } else {
                    0.0
                };

            // TODO: Diagonal will be faster! Fix once we have vectors
            let mut new_player_p = (*game_state).player_p.clone();
            new_player_p.offset_x += (*input).dt_for_frame * d_player_x;
            new_player_p.offset_y += (*input).dt_for_frame * d_player_y;
            new_player_p = recanonicalize_position(&(*tile_map), new_player_p);
            // TODO: Delta function that auto-recanonicalizes

            let mut player_left = new_player_p.clone();
            player_left.offset_x -= 0.5 * PLAYER_WIDTH;
            player_left = recanonicalize_position(&(*tile_map), player_left);

            let mut player_right = new_player_p.clone();
            player_right.offset_x += 0.5 * PLAYER_WIDTH;
            player_right = recanonicalize_position(&(*tile_map), player_right);

            if is_tile_map_point_empty(tile_map, &new_player_p)
                && is_tile_map_point_empty(tile_map, &player_left)
                && is_tile_map_point_empty(tile_map, &player_right)
            {
                if !are_on_same_tile(&(*game_state).player_p, &new_player_p) {
                    let new_tile_value = get_tile_value(tile_map, &new_player_p);

                    if new_tile_value == 3 {
                        new_player_p.abs_tile_z += 1;
                    } else if new_tile_value == 4 {
                        new_player_p.abs_tile_z -= 1;
                    }
                }
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

    let screen_center_x = 0.5 * (*buffer).width as f32;
    let screen_center_y = 0.5 * (*buffer).height as f32;

    for r in 0..20 {
        for c in 0..40 {
            let rel_row = r - 10;
            let rel_column = c - 20;
            let column = ((*game_state).player_p.abs_tile_x as i32 + rel_column) as u32;
            let row = ((*game_state).player_p.abs_tile_y as i32 + rel_row) as u32;
            let tile_id =
                get_tile_value_abs(tile_map, column, row, (*game_state).player_p.abs_tile_z);

            if tile_id > 0 {
                let gray = if tile_id == 2 {
                    1.0
                } else if column == (*game_state).player_p.abs_tile_x
                    && row == (*game_state).player_p.abs_tile_y
                {
                    0.0
                } else if tile_id > 2 {
                    0.25
                } else {
                    0.5
                };

                let cen_x = screen_center_x - meters_to_pixels * (*game_state).player_p.offset_x
                    + (rel_column * tile_side_in_pixels) as f32;
                let cen_y = screen_center_y + meters_to_pixels * (*game_state).player_p.offset_y
                    - (rel_row * tile_side_in_pixels) as f32;
                let min_x = cen_x - 0.5 * tile_side_in_pixels as f32;
                let min_y = cen_y - 0.5 * tile_side_in_pixels as f32;
                let max_x = cen_x + 0.5 * tile_side_in_pixels as f32;
                let max_y = cen_y + 0.5 * tile_side_in_pixels as f32;
                draw_rectangle(&(*buffer), min_x, min_y, max_x, max_y, gray, gray, gray);
            }
        }
    }

    let player_r = 1.0;
    let player_g = 1.0;
    let player_b = 0.0;
    let player_left = screen_center_x - 0.5 * meters_to_pixels * PLAYER_WIDTH;
    let player_top = screen_center_y - meters_to_pixels * PLAYER_HEIGHT;
    draw_rectangle(
        &(*buffer),
        player_left,
        player_top,
        player_left + meters_to_pixels * PLAYER_WIDTH,
        player_top + meters_to_pixels * PLAYER_HEIGHT,
        player_r,
        player_g,
        player_b,
    );

    /* not sure what Casey will do with this...

        uint32 *Source = GameState->PixelPointer;
        uint32 *Dest = (uint32 *)Buffer->Memory;
        for(int32 Y = 0;
            Y < Buffer->Height;
            ++Y)
        {
            for(int32 X = 0;
                X < Buffer->Width;
                ++X)
            {
                *Dest++ = *Source++;
            }
        }
    */
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
