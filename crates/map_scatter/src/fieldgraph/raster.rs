//! Raster storage for scalar field values.
//!
//! Stores samples over a chunked domain defined by [`ChunkGrid`].
use glam::Vec2;

use super::grid::ChunkGrid;

/// A raster grid with floating point values and a chunk grid for spatial reference.
#[derive(Clone, Debug)]
pub struct Raster {
    pub grid: ChunkGrid,
    pub data: Vec<f32>,
}

impl Raster {
    /// Create a new raster with the given chunk grid, initializing all values to zero.
    pub fn new(grid: ChunkGrid) -> Self {
        let len = grid.total_width() * grid.total_height();
        Self {
            grid,
            data: vec![0.0; len],
        }
    }

    /// Get the size of the raster as `(width, height)`, including halo regions.
    pub fn size(&self) -> (usize, usize) {
        (self.grid.total_width(), self.grid.total_height())
    }

    /// Get the value at the given grid indices, returning `0.0` if out of bounds.
    pub fn get(&self, ix: isize, iy: isize) -> f32 {
        let (w, h) = self.size();
        if ix < 0 || iy < 0 || ix >= w as isize || iy >= h as isize {
            return 0.0;
        }
        let i = (iy as usize) * w + (ix as usize);
        self.data[i]
    }

    /// Sample the raster at a world position, rounding to the nearest cell center.
    pub fn sample_domain(&self, p: Vec2) -> f32 {
        let (ix, iy) = self.grid.world_to_index(p);
        self.get(ix, iy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid() -> ChunkGrid {
        ChunkGrid {
            origin_domain: Vec2::new(0.0, 0.0),
            cell_size: 1.0,
            width: 2,
            height: 2,
            halo: 1,
        }
    }

    #[test]
    fn new_initializes_with_zeroes() {
        let grid = make_grid();
        let raster = Raster::new(grid.clone());
        assert_eq!(raster.size(), (4, 4));
        assert!(raster.data.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn get_returns_zero_outside_bounds() {
        let grid = make_grid();
        let raster = Raster::new(grid);
        assert_eq!(raster.get(-1, -1), 0.0);
        assert_eq!(raster.get(10, 10), 0.0);
    }

    #[test]
    fn sample_domain_uses_world_to_index() {
        let grid = make_grid();
        let mut raster = Raster::new(grid.clone());
        let idx = grid.world_to_index(Vec2::new(0.0, 0.0));
        let w = grid.total_width();
        raster.data[idx.1 as usize * w + idx.0 as usize] = 0.75;
        assert_eq!(raster.sample_domain(Vec2::new(0.0, 0.0)), 0.75);
    }
}
