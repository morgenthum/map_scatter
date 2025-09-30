//! Hexagonally-staggered jittered grid position sampling strategy.
use glam::Vec2;
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Hexagonally-staggered jittered grid sampling.
#[derive(Debug, Clone)]
pub struct HexJitterGridSampling {
    /// Jitter amount in the range [0, 1].
    /// 0 = no jitter (perfect lattice)
    /// 1 = max jitter (up to half the local spacing in each axis)
    pub jitter: f32,
    /// Base spacing along X for centers on the same row.
    pub cell_size: f32,
}

impl HexJitterGridSampling {
    /// Create a new hex jitter grid sampler with specified jitter (0.0 to 1.0)
    /// and base cell size along X.
    pub fn new(jitter: f32, cell_size: f32) -> Self {
        Self {
            jitter: jitter.clamp(0.0, 1.0),
            cell_size,
        }
    }
}

impl PositionSampling for HexJitterGridSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let w = domain_extent.x;
        let h = domain_extent.y;

        if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }

        // Effective spacing along X; ensure positive, finite
        let dx = if self.cell_size.is_finite() && self.cell_size > 0.0 {
            self.cell_size
        } else {
            (w.min(h) / 10.0).max(1.0)
        };

        // Hex/triangular lattice row spacing
        let dy = dx * (3.0_f32).sqrt() * 0.5;

        let mut cols = (w / dx).floor() as i32;
        let mut rows = (h / dy).floor() as i32;

        if cols < 1 {
            cols = 1;
        }
        if rows < 1 {
            rows = 1;
        }

        let cols = cols as usize;
        let rows = rows as usize;

        let half_w = w * 0.5;
        let half_h = h * 0.5;
        // Next representable floats below the right/top edges to enforce strict < comparisons
        let max_x = next_down(half_w);
        let max_y = next_down(half_h);

        // Jitter extents: up to half local spacing in each axis
        let jitter_x = self.jitter * (dx * 0.5);
        let jitter_y = self.jitter * (dy * 0.5);

        // Base centers
        let y0 = -half_h + 0.5 * dy;
        let x0_even = -half_w + 0.5 * dx;

        let mut points = Vec::with_capacity(cols * rows);

        for j in 0..rows {
            let y_c = y0 + (j as f32) * dy;

            // Per-row half-cell offset in x for odd rows
            let row_offset_x = if j % 2 == 0 { 0.0 } else { 0.5 * dx };
            let x0 = x0_even + row_offset_x;

            for i in 0..cols {
                let cx = x0 + (i as f32) * dx;
                let cy = y_c;

                // Apply per-cell jitter (uniform in [-jitter_*, jitter_*])
                let jx = if jitter_x > 0.0 {
                    let r = rand01(rng) * 2.0 - 1.0;
                    (r * jitter_x).clamp(-(dx * 0.5), dx * 0.5)
                } else {
                    0.0
                };
                let jy = if jitter_y > 0.0 {
                    let r = rand01(rng) * 2.0 - 1.0;
                    (r * jitter_y).clamp(-(dy * 0.5), dy * 0.5)
                } else {
                    0.0
                };

                let mut px = cx + jx;
                let mut py = cy + jy;

                // Keep strictly inside right/top edges to match convention elsewhere.
                px = px.clamp(-half_w, max_x);
                py = py.clamp(-half_h, max_y);

                points.push(Vec2::new(px, py));
            }
        }

        points.into_iter().map(Into::into).collect()
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn new_clamps_jitter_value() {
        let s = HexJitterGridSampling::new(2.5, 1.0);
        assert_eq!(s.jitter, 1.0);
        let s = HexJitterGridSampling::new(-0.5, 1.0);
        assert_eq!(s.jitter, 0.0);
    }

    #[test]
    fn generate_returns_empty_for_non_positive_extent() {
        let s = HexJitterGridSampling::new(0.0, 1.0);
        let mut rng = StdRng::seed_from_u64(1);
        assert!(s.generate(Vec2::new(0.0, 5.0).into(), &mut rng).is_empty());
        assert!(s.generate(Vec2::new(5.0, 0.0).into(), &mut rng).is_empty());
        assert!(s.generate(Vec2::new(-1.0, 1.0).into(), &mut rng).is_empty());
    }

    #[test]
    fn points_stay_inside_bounds() {
        let s = HexJitterGridSampling::new(1.0, 5.0);
        let mut rng = StdRng::seed_from_u64(42);
        let w = 23.0;
        let h = 17.0;
        let pts = s.generate(Vec2::new(w, h).into(), &mut rng);

        let half_w = w * 0.5;
        let half_h = h * 0.5;

        assert!(!pts.is_empty());
        for p in pts {
            assert!(p.x >= -half_w && p.x < half_w);
            assert!(p.y >= -half_h && p.y < half_h);
        }
    }

    #[test]
    fn odd_rows_are_offset_when_no_jitter() {
        let s = HexJitterGridSampling::new(0.0, 4.0);
        let mut rng = StdRng::seed_from_u64(7);
        // Ensure at least two rows/columns
        let pts = s.generate(Vec2::new(20.0, 20.0).into(), &mut rng);
        assert!(!pts.is_empty());

        // Infer dx and dy as in the implementation
        let dx = 4.0;
        let dy = dx * (3.0_f32).sqrt() * 0.5;

        // Build first two rows from the output by bucketing near y levels.
        // We accept small FP differences; just verify average x-offset between first two rows.
        // Extract y values and sort unique-ish by rounding to grid.
        let mut row0: Vec<f32> = Vec::new();
        let mut row1: Vec<f32> = Vec::new();

        // Find approximate min y to determine row banding
        let min_y = pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let band0_max = min_y + dy * 0.75;
        let band1_min = band0_max + dy * 0.25;

        for p in &pts {
            if p.y <= band0_max {
                row0.push(p.x);
            } else if p.y >= band1_min && p.y < band1_min + dy * 0.75 {
                row1.push(p.x);
            }
        }

        if !row0.is_empty() && !row1.is_empty() {
            row0.sort_by(|a, b| a.partial_cmp(b).unwrap());
            row1.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // Compare the first element (left-most) difference
            let dx_est = (row1[0] - row0[0]).abs();
            // Should be close to half a cell, within a tolerance
            assert!((dx_est - dx * 0.5).abs() < 0.6);
        }
    }
}
