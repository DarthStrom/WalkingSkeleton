use crate::common::*;

#[derive(Clone)]
pub struct TileMapPosition {
    // These are fixed point tile locations. The high
    // bits are the tile chunk index, and the low bits are the tile
    // index in the chunk.
    pub abs_tile_x: u32,
    pub abs_tile_y: u32,
    pub abs_tile_z: u32,

    // TODO: Should these be from the center of a tile?
    // TODO: Rename to offset x and y
    pub tile_rel_x: f32,
    pub tile_rel_y: f32,
}

#[derive(Clone)]
pub struct TileChunkPosition {
    tile_chunk_x: u32,
    tile_chunk_y: u32,
    tile_chunk_z: u32,

    rel_tile_x: u32,
    rel_tile_y: u32,
}

pub struct TileChunk {
    // TODO: Real structure for a tile
    pub tiles: *mut u32,
}

pub struct TileMap {
    pub chunk_shift: i32,
    pub chunk_mask: u32,
    pub chunk_dim: u32,

    pub tile_side_in_meters: f32,

    pub tile_chunk_count_x: u32,
    pub tile_chunk_count_y: u32,
    pub tile_chunk_count_z: u32,

    pub tile_chunks: *mut TileChunk,
}

pub fn recanonicalize_coord(tile_map: &TileMap, tile: &mut u32, tile_rel: &mut f32) {
    // TODO: Need to do something that doesn't use the divide/multiply method
    // for recanonicalizing because this can end up rounding back on to the tile
    // you just came from.

    // TileMap is assumed to bo toroidal topology, if you
    // step off one end you come back on the other
    let offset = (*tile_rel / tile_map.tile_side_in_meters).round() as i32;
    *tile = (*tile as i32 + offset) as u32;
    *tile_rel -= offset as f32 * tile_map.tile_side_in_meters;

    // TODO: Fix floating point math so this can be <
    debug_assert!(*tile_rel >= -0.5 * tile_map.tile_side_in_meters);
    debug_assert!(*tile_rel <= 0.5 * tile_map.tile_side_in_meters);
}

pub fn recanonicalize_position(tile_map: &TileMap, pos: TileMapPosition) -> TileMapPosition {
    let mut result = pos;

    recanonicalize_coord(tile_map, &mut result.abs_tile_x, &mut result.tile_rel_x);
    recanonicalize_coord(tile_map, &mut result.abs_tile_y, &mut result.tile_rel_y);

    result
}

unsafe fn get_tile_chunk(
    tile_map: *mut TileMap,
    tile_chunk_x: u32,
    tile_chunk_y: u32,
    tile_chunk_z: u32,
) -> Option<*mut TileChunk> {
    if tile_chunk_x < (*tile_map).tile_chunk_count_x
        && tile_chunk_y < (*tile_map).tile_chunk_count_y
        && tile_chunk_z < (*tile_map).tile_chunk_count_z
    {
        Some((*tile_map).tile_chunks.offset(
            (tile_chunk_z * (*tile_map).tile_chunk_count_y * (*tile_map).tile_chunk_count_x
                + tile_chunk_y * (*tile_map).tile_chunk_count_x
                + tile_chunk_x) as isize,
        ))
    } else {
        None
    }
}

unsafe fn get_tile_value(
    tile_map: &TileMap,
    tile_chunk: &TileChunk,
    tile_x: u32,
    tile_y: u32,
) -> u32 {
    debug_assert!(tile_x < tile_map.chunk_dim);
    debug_assert!(tile_y < tile_map.chunk_dim);

    if !tile_chunk.tiles.is_null() {
        *tile_chunk
            .tiles
            .offset((tile_y * tile_map.chunk_dim + tile_x) as isize)
    } else {
        0
    }
}

unsafe fn set_tile_value(
    tile_map: &TileMap,
    tile_chunk: &TileChunk,
    tile_x: u32,
    tile_y: u32,
    tile_value: u32,
) {
    debug_assert!(tile_x < tile_map.chunk_dim);
    debug_assert!(tile_y < tile_map.chunk_dim);

    *tile_chunk
        .tiles
        .offset((tile_y * tile_map.chunk_dim + tile_x) as isize) = tile_value;
}

fn get_chunk_position_for(
    tile_map: &TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
) -> TileChunkPosition {
    TileChunkPosition {
        tile_chunk_x: abs_tile_x >> tile_map.chunk_shift,
        tile_chunk_y: abs_tile_y >> tile_map.chunk_shift,
        tile_chunk_z: abs_tile_z,
        rel_tile_x: abs_tile_x & tile_map.chunk_mask,
        rel_tile_y: abs_tile_y & tile_map.chunk_mask,
    }
}

pub unsafe fn get_tile_value_abs(
    tile_map: *mut TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
) -> u32 {
    let chunk_pos = get_chunk_position_for(&(*tile_map), abs_tile_x, abs_tile_y, abs_tile_z);
    if let Some(tile_chunk) = get_tile_chunk(
        tile_map,
        chunk_pos.tile_chunk_x,
        chunk_pos.tile_chunk_y,
        chunk_pos.tile_chunk_z,
    ) {
        get_tile_value(
            &(*tile_map),
            &(*tile_chunk),
            chunk_pos.rel_tile_x,
            chunk_pos.rel_tile_y,
        )
    } else {
        0
    }
}

pub unsafe fn is_tile_map_point_empty(tile_map: *mut TileMap, can_pos: TileMapPosition) -> bool {
    get_tile_value_abs(
        tile_map,
        can_pos.abs_tile_x,
        can_pos.abs_tile_y,
        can_pos.abs_tile_z,
    ) == 1
}

pub unsafe fn set_tile_value_abs(
    arena: *mut MemoryArena,
    tile_map: *mut TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
    tile_value: u32,
) {
    let chunk_pos = get_chunk_position_for(&(*tile_map), abs_tile_x, abs_tile_y, abs_tile_z);
    let tile_chunk = get_tile_chunk(
        tile_map,
        chunk_pos.tile_chunk_x,
        chunk_pos.tile_chunk_y,
        chunk_pos.tile_chunk_z,
    )
    .expect("could not get tile chunk");

    if (*tile_chunk).tiles.is_null() {
        let tile_count = ((*tile_map).chunk_dim * (*tile_map).chunk_dim) as usize;
        (*tile_chunk).tiles = push_array::<u32>(arena, tile_count);
        for tile_index in 0..tile_count {
            (*(*tile_chunk).tiles.add(tile_index)) = 1;
        }
    }

    set_tile_value(
        &(*tile_map),
        &(*tile_chunk),
        chunk_pos.rel_tile_x,
        chunk_pos.rel_tile_y,
        tile_value,
    );
}
