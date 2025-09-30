//! Chunk utilities for the scatter pipeline.
//!
//! Convert between world-space positions and chunk/grid coordinates used during
//! evaluation and placement.
//!
//! Primary helper: [`chunk_id_and_grid_for_position_centered`], which yields both
//! [`ChunkId`] and [`ChunkGrid`] for a given world position and domain extent.
use glam::Vec2;

use crate::fieldgraph::{ChunkGrid, ChunkId};

/// Given a domain extent, returns the min and max coordinates for a centered domain.
pub fn domain_bounds_centered(domain_extent: Vec2) -> (Vec2, Vec2) {
    let half = domain_extent / 2.0;
    (-half, half)
}

/// Computes the chunk ID for a given world position, world minimum, and chunk size.
pub fn chunk_id_for_position(position: Vec2, world_min: Vec2, chunk_size: f32) -> ChunkId {
    debug_assert!(chunk_size > 0.0, "chunk_size must be > 0");
    let rel_x = position.x - world_min.x;
    let rel_y = position.y - world_min.y;
    let ix = (rel_x / chunk_size).floor() as i32;
    let iy = (rel_y / chunk_size).floor() as i32;
    ChunkId(ix, iy)
}

/// Computes the chunk ID for a given world position in a centered domain.
pub fn chunk_id_for_position_centered(
    position: Vec2,
    domain_extent: Vec2,
    chunk_size: f32,
) -> ChunkId {
    let (world_min, _) = domain_bounds_centered(domain_extent);
    chunk_id_for_position(position, world_min, chunk_size)
}

/// Computes the world origin of a chunk given its ID, world minimum, and chunk size.
pub fn chunk_origin_for_chunk_id(world_min: Vec2, chunk_size: f32, idx: ChunkId) -> Vec2 {
    debug_assert!(chunk_size > 0.0, "chunk_size must be > 0");
    world_min + Vec2::new(idx.0 as f32, idx.1 as f32) * chunk_size
}

/// Computes the world origin of a chunk given its ID in a centered domain.
pub fn chunk_origin_for_chunk_id_centered(
    domain_extent: Vec2,
    chunk_size: f32,
    idx: ChunkId,
) -> Vec2 {
    let (world_min, _) = domain_bounds_centered(domain_extent);
    chunk_origin_for_chunk_id(world_min, chunk_size, idx)
}

/// Computes the grid dimensions `(width, height)` for a chunk given its size and raster cell size.
pub fn grid_dims_for_chunk(chunk_size: f32, raster_cell_size: f32) -> (usize, usize) {
    debug_assert!(chunk_size > 0.0, "chunk_size must be > 0");
    debug_assert!(raster_cell_size > 0.0, "raster_cell_size must be > 0");
    let w = (chunk_size / raster_cell_size).ceil().max(1.0) as usize;
    let h = (chunk_size / raster_cell_size).ceil().max(1.0) as usize;
    (w, h)
}

/// Creates a [`ChunkGrid`] for a given [`ChunkId`], world minimum, chunk size,
/// raster cell size, and halo.
pub fn make_chunk_grid(
    world_min: Vec2,
    chunk_size: f32,
    raster_cell_size: f32,
    halo: usize,
    idx: ChunkId,
) -> ChunkGrid {
    let origin_domain = chunk_origin_for_chunk_id(world_min, chunk_size, idx);
    let (width, height) = grid_dims_for_chunk(chunk_size, raster_cell_size);
    ChunkGrid {
        origin_domain,
        cell_size: raster_cell_size,
        width,
        height,
        halo,
    }
}

/// Creates a [`ChunkGrid`] for a given [`ChunkId`] in a centered domain.
pub fn make_chunk_grid_centered(
    domain_extent: Vec2,
    chunk_size: f32,
    raster_cell_size: f32,
    halo: usize,
    idx: ChunkId,
) -> ChunkGrid {
    let (world_min, _) = domain_bounds_centered(domain_extent);
    make_chunk_grid(world_min, chunk_size, raster_cell_size, halo, idx)
}

/// Computes both the [`ChunkId`] and corresponding [`ChunkGrid`] for a given world position.
pub fn chunk_id_and_grid_for_position_centered(
    position: Vec2,
    domain_extent: Vec2,
    chunk_size: f32,
    raster_cell_size: f32,
    halo: usize,
) -> (ChunkId, ChunkGrid) {
    let (world_min, _) = domain_bounds_centered(domain_extent);
    let idx = chunk_id_for_position(position, world_min, chunk_size);
    let grid = make_chunk_grid(world_min, chunk_size, raster_cell_size, halo, idx);
    (idx, grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_bounds_centered_symmetric() {
        let (min, max) = domain_bounds_centered(Vec2::new(10.0, 6.0));
        assert_eq!(min, Vec2::new(-5.0, -3.0));
        assert_eq!(max, Vec2::new(5.0, 3.0));
    }

    #[test]
    fn chunk_id_computations_match_inverse() {
        let world_min = Vec2::new(-5.0, -5.0);
        let chunk_size = 2.0;
        let position = Vec2::new(-3.5, -1.0);
        let id = chunk_id_for_position(position, world_min, chunk_size);
        assert_eq!(id, ChunkId(0, 2));

        let origin = chunk_origin_for_chunk_id(world_min, chunk_size, id);
        assert_eq!(origin, Vec2::new(-5.0, -1.0));
    }

    #[test]
    fn centered_variants_delegate_to_helpers() {
        let domain = Vec2::new(8.0, 8.0);
        let chunk_size = 4.0;
        let position = Vec2::new(1.0, 1.0);
        let id_centered = chunk_id_for_position_centered(position, domain, chunk_size);
        let (id, grid) =
            chunk_id_and_grid_for_position_centered(position, domain, chunk_size, 1.0, 1);
        assert_eq!(id, id_centered);
        assert_eq!(grid.width, 4);
        assert_eq!(grid.height, 4);
        assert_eq!(grid.halo, 1);
    }

    #[test]
    fn make_chunk_grid_sets_dimensions() {
        let grid = make_chunk_grid(Vec2::new(0.0, 0.0), 3.0, 1.0, 2, ChunkId(1, 1));
        assert_eq!(grid.width, 3);
        assert_eq!(grid.height, 3);
        assert_eq!(grid.halo, 2);
        assert_eq!(grid.origin_domain, Vec2::new(3.0, 3.0));
    }
}
