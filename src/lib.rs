//! equivalent to handmade.h & handmade.cpp

pub mod common;
mod tile;

use common::*;
use core::mem::*;
use image::{DynamicImage, GenericImageView};
use rand::prelude::*;
use tile::*;

#[macro_use]
extern crate log;

use std::f32;

struct World {
    tile_map: TileMap,
}

struct CharacterImage {
    align_x: i32,
    align_y: i32,
    image: DynamicImage,
    frame_width: u32,
    frames: u32,
}

struct State {
    world_arena: MemoryArena,
    world: *mut World,

    camera_p: TileMapPosition,
    player_p: TileMapPosition,

    backdrop: DynamicImage,

    character_image: CharacterImage,
    character_walk_frame: u32,
}

/// This ensures that GameUpdateAndRender has a signature that will match what
/// is specified in handmade_platform.rs
const _UPDATE_CHECK: GameUpdateAndRender = update_and_render;

const PLAYER_HEIGHT: f32 = 1.4;
const PLAYER_WIDTH: f32 = 0.75 * PLAYER_HEIGHT;

// TODO: platform independent code should get priority for removing unsafe
fn initialize_arena(arena: &mut MemoryArena, size: usize, base: *mut u8) {
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
        (*game_state).backdrop =
            image::open("data/assets/Bricks.png").expect("could not load background");

        (*game_state).character_image = CharacterImage {
            image: image::open("data/assets/Skeleton Walk.png")
                .expect("could not load skeleton walk png"),
            align_x: 10,
            align_y: 33,
            frame_width: 22,
            frames: 13,
        };
        (*game_state).camera_p.abs_tile_x = 17 / 2;
        (*game_state).camera_p.abs_tile_y = 9 / 2;

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

        let tile_map = &mut (*world).tile_map;

        tile_map.chunk_shift = 4;
        tile_map.chunk_mask = (1 << tile_map.chunk_shift) - 1;
        tile_map.chunk_dim = 1 << tile_map.chunk_shift;

        tile_map.tile_chunk_count_x = 128;
        tile_map.tile_chunk_count_y = 128;
        tile_map.tile_chunk_count_z = 2;
        let tile_chunk_count =
            tile_map.tile_chunk_count_x * tile_map.tile_chunk_count_y * tile_map.tile_chunk_count_z;
        for _ in 0..tile_chunk_count {
            tile_map.tile_chunks.push(TileChunk { tiles: vec![] })
        }

        tile_map.tile_side_in_meters = 1.4;

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
        for _screen_index in 0..100 {
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

                    set_tile_value(tile_map, abs_tile_x, abs_tile_y, abs_tile_z, tile_value);
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
    let tile_map = &mut (*world).tile_map;

    let tile_side_in_pixels = 60;
    let meters_to_pixels = tile_side_in_pixels as f32 / tile_map.tile_side_in_meters;

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
                    (*game_state).character_walk_frame += 1;
                    (*game_state).character_walk_frame %= (*game_state).character_image.frames;
                    1.0
                } else if (*controller).move_down.ended_down {
                    (*game_state).character_walk_frame += 1;
                    (*game_state).character_walk_frame %= (*game_state).character_image.frames;
                    -1.0
                } else {
                    0.0
                };
            let d_player_x = player_speed
                * if (*controller).move_left.ended_down {
                    (*game_state).character_walk_frame += 1;
                    (*game_state).character_walk_frame %= (*game_state).character_image.frames;
                    -1.0
                } else if (*controller).move_right.ended_down {
                    (*game_state).character_walk_frame += 1;
                    (*game_state).character_walk_frame %= (*game_state).character_image.frames;
                    1.0
                } else {
                    0.0
                };

            // TODO: Diagonal will be faster! Fix once we have vectors
            let mut new_player_p = (*game_state).player_p.clone();
            new_player_p.offset_x += (*input).dt_for_frame * d_player_x;
            new_player_p.offset_y += (*input).dt_for_frame * d_player_y;
            new_player_p = recanonicalize_position(tile_map, new_player_p);
            // TODO: Delta function that auto-recanonicalizes

            let mut player_left = new_player_p.clone();
            player_left.offset_x -= 0.5 * PLAYER_WIDTH;
            player_left = recanonicalize_position(tile_map, player_left);

            let mut player_right = new_player_p.clone();
            player_right.offset_x += 0.5 * PLAYER_WIDTH;
            player_right = recanonicalize_position(tile_map, player_right);

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

            (*game_state).camera_p.abs_tile_z = (*game_state).player_p.abs_tile_z;

            let diff = subtract(tile_map, &(*game_state).player_p, &(*game_state).camera_p);
            if diff.dx > (9.0 * tile_map.tile_side_in_meters) {
                (*game_state).camera_p.abs_tile_x += 17;
            }
            if diff.dx < -(9.0 * tile_map.tile_side_in_meters) {
                (*game_state).camera_p.abs_tile_x -= 17;
            }
            if diff.dy > (5.0 * tile_map.tile_side_in_meters) {
                (*game_state).camera_p.abs_tile_y += 9;
            }
            if diff.dy < -(5.0 * tile_map.tile_side_in_meters) {
                (*game_state).camera_p.abs_tile_y -= 9;
            }
        }
    }

    draw_image(
        &(*buffer),
        &(*game_state).backdrop,
        0.0,
        0.0,
        (*game_state).backdrop.width(),
        0,
    );

    let screen_center_x = 0.5 * (*buffer).width as f32;
    let screen_center_y = 0.5 * (*buffer).height as f32;

    for r in 0..20 {
        for c in 0..40 {
            let rel_row = r - 10;
            let rel_column = c - 20;
            let column = ((*game_state).camera_p.abs_tile_x as i32 + rel_column) as u32;
            let row = ((*game_state).camera_p.abs_tile_y as i32 + rel_row) as u32;
            let tile_id =
                get_tile_value_abs(tile_map, column, row, (*game_state).camera_p.abs_tile_z);

            if tile_id > 1 {
                let gray = if tile_id == 2 {
                    1.0
                } else if column == (*game_state).camera_p.abs_tile_x
                    && row == (*game_state).camera_p.abs_tile_y
                {
                    0.0
                } else if tile_id > 2 {
                    0.25
                } else {
                    0.5
                };

                let cen_x = screen_center_x - meters_to_pixels * (*game_state).camera_p.offset_x
                    + (rel_column * tile_side_in_pixels) as f32;
                let cen_y = screen_center_y + meters_to_pixels * (*game_state).camera_p.offset_y
                    - (rel_row * tile_side_in_pixels) as f32;
                let min_x = cen_x - 0.5 * tile_side_in_pixels as f32;
                let min_y = cen_y - 0.5 * tile_side_in_pixels as f32;
                let max_x = cen_x + 0.5 * tile_side_in_pixels as f32;
                let max_y = cen_y + 0.5 * tile_side_in_pixels as f32;
                draw_rectangle(&(*buffer), min_x, min_y, max_x, max_y, gray, gray, gray);
            }
        }
    }

    let diff = subtract(tile_map, &(*game_state).player_p, &(*game_state).camera_p);

    let player_ground_point_x = screen_center_x + meters_to_pixels * diff.dx;
    let player_ground_point_y = screen_center_y - meters_to_pixels * diff.dy;

    let character_image = &(*game_state).character_image;
    draw_animated_image(
        &(*buffer),
        &character_image.image,
        player_ground_point_x,
        player_ground_point_y,
        character_image.align_x,
        character_image.align_y,
        character_image.frame_width,
        (*game_state).character_walk_frame,
    );
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

fn draw_animated_image(
    buffer: &GameOffscreenBuffer,
    bitmap: &DynamicImage,
    real_x: f32,
    real_y: f32,
    align_x: i32,
    align_y: i32,
    frame_width: u32,
    frame: u32,
) {
    let x = real_x - align_x as f32;
    let y = real_y - align_y as f32;

    draw_image(
        buffer,
        bitmap,
        x,
        y,
        frame_width,
        (frame * frame_width) as i32,
    )
}

fn draw_image(
    buffer: &GameOffscreenBuffer,
    bitmap: &DynamicImage,
    real_x: f32,
    real_y: f32,
    width: u32,
    x_offset: i32,
) {
    let mut min_x = real_x.round() as i32;
    let mut min_y = real_y.round() as i32;
    let mut max_x = real_x as i32 + width as i32;
    let mut max_y = real_y as i32 + bitmap.height() as i32;

    let mut source_offset_x = 0;
    if min_x < 0 {
        source_offset_x = -min_x;
        min_x = 0;
    }
    source_offset_x += x_offset;

    let mut source_offset_y = 0;
    if min_y < 0 {
        source_offset_y = -min_y;
        min_y = 0;
    }

    if max_x > buffer.width {
        max_x = buffer.width;
    }

    if max_y > buffer.height {
        max_y = buffer.height;
    }

    unsafe {
        let mut dest_row = (buffer.memory as *mut u8)
            .offset((min_x * buffer.bytes_per_pixel + min_y * buffer.pitch) as isize);
        for y in min_y..max_y {
            #[allow(clippy::cast_ptr_alignment)]
            let mut dest = dest_row as *mut u32;
            for x in min_x..max_x {
                let source_x = (source_offset_x + x - min_x) as u32;
                let source_y = (source_offset_y + y - min_y) as u32;

                if source_x < bitmap.width() && source_y < bitmap.height() {
                    let pixel = bitmap.get_pixel(source_x, source_y);

                    let a = pixel[3] as f32 / 255.0;
                    let sr = pixel[0] as f32;
                    let sg = pixel[1] as f32;
                    let sb = pixel[2] as f32;

                    let dr = ((*dest >> 16) & 0xFF) as f32;
                    let dg = ((*dest >> 8) & 0xFF) as f32;
                    let db = (*dest & 0xFF) as f32;

                    // TODO: Investigate premultiplied alpha
                    let r = (1.0 - a) * dr + a * sr;
                    let g = (1.0 - a) * dg + a * sg;
                    let b = (1.0 - a) * db + a * sb;

                    *dest = (((r + 0.5) as u32) << 16) | ((g + 0.5) as u32) << 8 | (b + 0.5) as u32;
                }

                dest = dest.add(1);
            }

            dest_row = dest_row.offset(buffer.pitch as isize);
        }
    }
}
