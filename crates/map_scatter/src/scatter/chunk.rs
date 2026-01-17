//! Chunk utilities for the scatter pipeline.
//!
//! Convert between world-space positions and chunk/grid coordinates used during
//! evaluation and placement.
//!
//! Primary helper: [`chunk_id_and_grid_for_position_centered`], which yields both
//! [`ChunkId`] and [`ChunkGrid`] for a given world position and domain extent.
use glam::Vec2;

use crate::fieldgraph::{ChunkGrid, ChunkId};

/// Given a domain extent and center, returns the min and max coordinates for the domain.
pub fn domain_bounds(domain_extent: Vec2, domain_center: Vec2) -> (Vec2, Vec2) {
    let half = domain_extent / 2.0;
    (domain_center - half, domain_center + half)
}

/// Given a domain extent, returns the min and max coordinates for a centered domain.
pub fn domain_bounds_centered(domain_extent: Vec2) -> (Vec2, Vec2) {
    domain_bounds(domain_extent, Vec2::ZERO)
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

/// Computes the chunk ID for a given world position in a domain with custom center.
pub fn chunk_id_for_position_in_domain(
    position: Vec2,
    domain_extent: Vec2,
    domain_center: Vec2,
    chunk_size: f32,
) -> ChunkId {
    let (world_min, _) = domain_bounds(domain_extent, domain_center);
    chunk_id_for_position(position, world_min, chunk_size)
}

/// Computes the chunk ID for a given world position in a centered domain.
pub fn chunk_id_for_position_centered(
    position: Vec2,
    domain_extent: Vec2,
    chunk_size: f32,
) -> ChunkId {
    chunk_id_for_position_in_domain(position, domain_extent, Vec2::ZERO, chunk_size)
}

/// Computes the world origin of a chunk given its ID, world minimum, and chunk size.
pub fn chunk_origin_for_chunk_id(world_min: Vec2, chunk_size: f32, idx: ChunkId) -> Vec2 {
    debug_assert!(chunk_size > 0.0, "chunk_size must be > 0");
    world_min + Vec2::new(idx.0 as f32, idx.1 as f32) * chunk_size
}

/// Computes the world origin of a chunk given its ID in a domain with custom center.
pub fn chunk_origin_for_chunk_id_in_domain(
    domain_extent: Vec2,
    domain_center: Vec2,
    chunk_size: f32,
    idx: ChunkId,
) -> Vec2 {
    let (world_min, _) = domain_bounds(domain_extent, domain_center);
    chunk_origin_for_chunk_id(world_min, chunk_size, idx)
}

/// Computes the world origin of a chunk given its ID in a centered domain.
pub fn chunk_origin_for_chunk_id_centered(
    domain_extent: Vec2,
    chunk_size: f32,
    idx: ChunkId,
) -> Vec2 {
    chunk_origin_for_chunk_id_in_domain(domain_extent, Vec2::ZERO, chunk_size, idx)
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

/// Creates a [`ChunkGrid`] for a given [`ChunkId`] in a domain with custom center.
pub fn make_chunk_grid_in_domain(
    domain_extent: Vec2,
    domain_center: Vec2,
    chunk_size: f32,
    raster_cell_size: f32,
    halo: usize,
    idx: ChunkId,
) -> ChunkGrid {
    let (world_min, _) = domain_bounds(domain_extent, domain_center);
    make_chunk_grid(world_min, chunk_size, raster_cell_size, halo, idx)
}

/// Creates a [`ChunkGrid`] for a given [`ChunkId`] in a centered domain.
pub fn make_chunk_grid_centered(
    domain_extent: Vec2,
    chunk_size: f32,
    raster_cell_size: f32,
    halo: usize,
    idx: ChunkId,
) -> ChunkGrid {
    make_chunk_grid_in_domain(
        domain_extent,
        Vec2::ZERO,
        chunk_size,
        raster_cell_size,
        halo,
        idx,
    )
}

/// Computes both the [`ChunkId`] and corresponding [`ChunkGrid`] for a given world position.
pub fn chunk_id_and_grid_for_position_centered(
    position: Vec2,
    domain_extent: Vec2,
    chunk_size: f32,
    raster_cell_size: f32,
    halo: usize,
) -> (ChunkId, ChunkGrid) {
    chunk_id_and_grid_for_position_in_domain(
        position,
        domain_extent,
        Vec2::ZERO,
        chunk_size,
        raster_cell_size,
        halo,
    )
}

/// Computes both the [`ChunkId`] and corresponding [`ChunkGrid`] for a given world position
/// in a domain with custom center.
pub fn chunk_id_and_grid_for_position_in_domain(
    position: Vec2,
    domain_extent: Vec2,
    domain_center: Vec2,
    chunk_size: f32,
    raster_cell_size: f32,
    halo: usize,
) -> (ChunkId, ChunkGrid) {
    let (world_min, _) = domain_bounds(domain_extent, domain_center);
    let idx = chunk_id_for_position(position, world_min, chunk_size);
    let grid = make_chunk_grid(world_min, chunk_size, raster_cell_size, halo, idx);
    (idx, grid)
}

/// Creates a deterministic seed for a chunk from a base seed.
pub fn seed_for_chunk(base_seed: u64, chunk: ChunkId) -> u64 {
    let cx = chunk.0 as i64 as u64;
    let cy = chunk.1 as i64 as u64;
    let mixed =
        base_seed ^ cx.wrapping_mul(0x9E3779B97F4A7C15) ^ cy.wrapping_mul(0xBF58476D1CE4E5B9);
    mix_u64(mixed)
}

#[inline]
fn mix_u64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
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

    #[test]
    fn domain_bounds_respects_center() {
        let (min, max) = domain_bounds(Vec2::new(4.0, 2.0), Vec2::new(10.0, -5.0));
        assert_eq!(min, Vec2::new(8.0, -6.0));
        assert_eq!(max, Vec2::new(12.0, -4.0));
    }
}
