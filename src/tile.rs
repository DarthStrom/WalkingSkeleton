pub struct TileMapDifference {
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
}

#[derive(Clone)]
pub struct TileMapPosition {
    // These are fixed point tile locations. The high
    // bits are the tile chunk index, and the low bits are the tile
    // index in the chunk.
    pub abs_tile_x: u32,
    pub abs_tile_y: u32,
    pub abs_tile_z: u32,

    // These are the offsets from the tile center
    pub offset_x: f32,
    pub offset_y: f32,
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
    pub tiles: Vec<u32>,
}

pub struct TileMap {
    pub chunk_shift: u32,
    pub chunk_mask: u32,
    pub chunk_dim: u32,

    pub tile_side_in_meters: f32,

    pub tile_chunk_count_x: u32,
    pub tile_chunk_count_y: u32,
    pub tile_chunk_count_z: u32,

    pub tile_chunks: Vec<TileChunk>,
}

fn get_tile_chunk(
    tile_map: &TileMap,
    tile_chunk_x: u32,
    tile_chunk_y: u32,
    tile_chunk_z: u32,
) -> Option<&TileChunk> {
    if let Some(index) = get_chunk_index(tile_map, tile_chunk_x, tile_chunk_y, tile_chunk_z) {
        tile_map.tile_chunks.get(index)
    } else {
        None
    }
}

fn get_tile_chunk_mut(
    tile_map: &mut TileMap,
    tile_chunk_x: u32,
    tile_chunk_y: u32,
    tile_chunk_z: u32,
) -> Option<&mut TileChunk> {
    if let Some(index) = get_chunk_index(tile_map, tile_chunk_x, tile_chunk_y, tile_chunk_z) {
        tile_map.tile_chunks.get_mut(index)
    } else {
        None
    }
}

fn get_chunk_index(
    tile_map: &TileMap,
    tile_chunk_x: u32,
    tile_chunk_y: u32,
    tile_chunk_z: u32,
) -> Option<usize> {
    if tile_chunk_x < tile_map.tile_chunk_count_x
        && tile_chunk_y < tile_map.tile_chunk_count_y
        && tile_chunk_z < tile_map.tile_chunk_count_z
    {
        Some(
            (tile_chunk_z * tile_map.tile_chunk_count_y * tile_map.tile_chunk_count_x
                + tile_chunk_y * tile_map.tile_chunk_count_x
                + tile_chunk_x) as usize,
        )
    } else {
        None
    }
}

fn get_tile_value_rel(
    tile_map: &TileMap,
    tile_chunk: &TileChunk,
    tile_x: u32,
    tile_y: u32,
) -> u32 {
    debug_assert!(tile_x < tile_map.chunk_dim);
    debug_assert!(tile_y < tile_map.chunk_dim);

    if let Some(value) = tile_chunk.tiles.get((tile_y * tile_map.chunk_dim + tile_x) as usize) {
        *value
    } else {
        0
    }
}

fn set_tile_value_for_chunk(
    chunk_dim: u32,
    tile_chunk: &mut TileChunk,
    tile_x: u32,
    tile_y: u32,
    tile_value: u32,
) {
    debug_assert!(tile_x < chunk_dim);
    debug_assert!(tile_y < chunk_dim);

    tile_chunk.tiles[(tile_y * chunk_dim + tile_x) as usize] = tile_value
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

pub fn get_tile_value_abs(
    tile_map: &TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
) -> u32 {
    let chunk_pos = get_chunk_position_for(tile_map, abs_tile_x, abs_tile_y, abs_tile_z);
    if let Some(tile_chunk) = get_tile_chunk(
        tile_map,
        chunk_pos.tile_chunk_x,
        chunk_pos.tile_chunk_y,
        chunk_pos.tile_chunk_z,
    ) {
        get_tile_value_rel(
            tile_map,
            tile_chunk,
            chunk_pos.rel_tile_x,
            chunk_pos.rel_tile_y,
        )
    } else {
        0
    }
}

pub fn get_tile_value(tile_map: &TileMap, pos: &TileMapPosition) -> u32 {
    get_tile_value_abs(tile_map, pos.abs_tile_x, pos.abs_tile_y, pos.abs_tile_z)
}

pub fn is_tile_map_point_empty(tile_map: &TileMap, pos: &TileMapPosition) -> bool {
    // TODO match an enum for this
    [1, 3, 4].contains(&get_tile_value(tile_map, pos))
}

pub fn set_tile_value(
    tile_map: &mut TileMap,
    abs_tile_x: u32,
    abs_tile_y: u32,
    abs_tile_z: u32,
    tile_value: u32,
) {
    let chunk_pos = get_chunk_position_for(tile_map, abs_tile_x, abs_tile_y, abs_tile_z);
    let chunk_dim = tile_map.chunk_dim;
    let tile_chunk = get_tile_chunk_mut(
        tile_map,
        chunk_pos.tile_chunk_x,
        chunk_pos.tile_chunk_y,
        chunk_pos.tile_chunk_z,
    )
    .expect("could not get tile_chunk");

    if tile_chunk.tiles.is_empty() {
        let tile_count = chunk_dim * chunk_dim;
        for _ in 0..tile_count {
            tile_chunk.tiles.push(1);
        }
    }

    set_tile_value_for_chunk(
        chunk_dim,
        tile_chunk,
        chunk_pos.rel_tile_x,
        chunk_pos.rel_tile_y,
        tile_value,
    );
}

//
// TODO: Do these really belong in more of a "positioning" or "geometry" file?
//

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

    recanonicalize_coord(tile_map, &mut result.abs_tile_x, &mut result.offset_x);
    recanonicalize_coord(tile_map, &mut result.abs_tile_y, &mut result.offset_y);

    result
}

pub fn are_on_same_tile(a: &TileMapPosition, b: &TileMapPosition) -> bool {
    a.abs_tile_x == b.abs_tile_x && a.abs_tile_y == b.abs_tile_y && a.abs_tile_z == b.abs_tile_z
}

pub fn subtract(tile_map: &TileMap, a: &TileMapPosition, b: &TileMapPosition) -> TileMapDifference {
    let d_tile_x = a.abs_tile_x as f32 - b.abs_tile_x as f32;
    let d_tile_y = a.abs_tile_y as f32 - b.abs_tile_y as f32;
    let d_tile_z = a.abs_tile_z as f32 - b.abs_tile_z as f32;

    TileMapDifference {
        dx: tile_map.tile_side_in_meters * d_tile_x + (a.offset_x - b.offset_x),
        dy: tile_map.tile_side_in_meters * d_tile_y + (a.offset_y - b.offset_y),
        // TODO: think about what to do about z
        dz: tile_map.tile_side_in_meters * d_tile_z,
    }
}
