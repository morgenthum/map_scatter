//! Chunked grid utilities for spatial evaluation.
//!
//! This module defines [`ChunkGrid`] and [`ChunkId`] to partition a 2D domain into chunks
//! with optional halo cells. It is used by the fieldgraph runtime and raster baking paths.
use glam::Vec2;

/// Identifier for a chunk in the chunk grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkId(
    /// Chunk index along the X axis.
    pub i32,
    /// Chunk index along the Y axis.
    pub i32,
);

/// Defines a 2D grid of cells with halo regions for chunked processing.
#[derive(Clone, Debug)]
pub struct ChunkGrid {
    /// World-space origin of the chunk grid (lower-left corner).
    pub origin_domain: Vec2,
    /// Cell size in world units.
    pub cell_size: f32,
    /// Number of cells in X, excluding halo.
    pub width: usize,
    /// Number of cells in Y, excluding halo.
    pub height: usize,
    /// Halo cell count on each side.
    pub halo: usize,
}

impl ChunkGrid {
    /// Total width including halo regions.
    pub fn total_width(&self) -> usize {
        self.width + 2 * self.halo
    }

    /// Total height including halo regions.
    pub fn total_height(&self) -> usize {
        self.height + 2 * self.halo
    }

    /// Converts a world position to grid cell indices, accounting for halo.
    pub fn world_to_index(&self, p: Vec2) -> (isize, isize) {
        let px = (p.x - self.origin_domain.x) / self.cell_size + self.halo as f32;
        let py = (p.y - self.origin_domain.y) / self.cell_size + self.halo as f32;
        (px.floor() as isize, py.floor() as isize)
    }

    /// Converts grid cell indices back to world position at the cell center, accounting for halo.
    pub fn index_to_world(&self, ix: isize, iy: isize) -> Vec2 {
        Vec2::new(
            self.origin_domain.x + (ix as f32 - self.halo as f32) * self.cell_size,
            self.origin_domain.y + (iy as f32 - self.halo as f32) * self.cell_size,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_grid() -> ChunkGrid {
        ChunkGrid {
            origin_domain: Vec2::new(-5.0, -5.0),
            cell_size: 1.0,
            width: 4,
            height: 3,
            halo: 1,
        }
    }

    #[test]
    fn total_dimensions_include_halo() {
        let grid = sample_grid();
        assert_eq!(grid.total_width(), 6);
        assert_eq!(grid.total_height(), 5);
    }

    #[test]
    fn world_index_roundtrip() {
        let grid = sample_grid();
        let p = Vec2::new(-5.0, -5.0);
        let (ix, iy) = grid.world_to_index(p);
        assert_eq!((ix, iy), (1, 1));

        let back = grid.index_to_world(ix, iy);
        assert_eq!(back, Vec2::new(-5.0, -5.0));
    }
}
