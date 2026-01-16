//! Jittered-grid position sampling strategy.
use glam::Vec2;
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Jittered grid sampling.
#[derive(Debug, Clone)]
pub struct JitterGridSampling {
    /// Jitter amount in [0, 1], where 0 is grid centers and 1 is max jitter.
    pub jitter: f32,
    /// Cell size for the grid.
    pub cell_size: f32,
}

impl JitterGridSampling {
    /// Create a new JitterGridSampling with specified jitter (0.0 to 1.0).
    pub fn new(jitter: f32, cell_size: f32) -> Self {
        Self {
            jitter: jitter.clamp(0.0, 1.0),
            cell_size,
        }
    }
}

impl PositionSampling for JitterGridSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let domain_extent = Vec2::from(domain_extent);
        if domain_extent.x <= 0.0 || domain_extent.y <= 0.0 {
            return Vec::new();
        }

        let w = domain_extent.x;
        let h = domain_extent.y;
        let eff = if self.cell_size.is_finite() && self.cell_size > 0.0 {
            self.cell_size
        } else {
            (w.min(h) / 10.0).max(1.0)
        };

        let mut cols = (w / eff).floor() as i32;
        let mut rows = (h / eff).floor() as i32;

        if cols < 1 {
            cols = 1;
        }
        if rows < 1 {
            rows = 1;
        }

        let cols = cols as usize;
        let rows = rows as usize;

        let cell_w = w / cols as f32;
        let cell_h = h / rows as f32;

        let half_w = w * 0.5;
        let half_h = h * 0.5;

        let max_x = next_down(half_w);
        let max_y = next_down(half_h);

        let jitter_x = self.jitter * (cell_w * 0.5);
        let jitter_y = self.jitter * (cell_h * 0.5);

        let mut points = Vec::with_capacity(cols * rows);

        for j in 0..rows {
            for i in 0..cols {
                let x0 = -half_w + i as f32 * cell_w;
                let y0 = -half_h + j as f32 * cell_h;
                let cx = x0 + cell_w * 0.5;
                let cy = y0 + cell_h * 0.5;
                let jx = if jitter_x > 0.0 {
                    let r = rand01(rng) * 2.0 - 1.0;
                    (r * jitter_x).clamp(-(cell_w * 0.5), cell_w * 0.5)
                } else {
                    0.0
                };
                let jy = if jitter_y > 0.0 {
                    let r = rand01(rng) * 2.0 - 1.0;
                    (r * jitter_y).clamp(-(cell_h * 0.5), cell_h * 0.5)
                } else {
                    0.0
                };
                let mut px = cx + jx;
                let mut py = cy + jy;
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
        let sampler = JitterGridSampling::new(2.0, 1.0);
        assert_eq!(sampler.jitter, 1.0);
    }

    #[test]
    fn generate_returns_grid_centers_without_jitter() {
        let strategy = JitterGridSampling::new(0.0, 2.0);
        let mut rng = StdRng::seed_from_u64(1);
        let points = strategy.generate(Vec2::new(4.0, 4.0).into(), &mut rng);

        assert_eq!(points.len(), 4);
        let mut xs: Vec<_> = points.iter().map(|p| p.x).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(xs, vec![-1.0, -1.0, 1.0, 1.0]);

        let mut ys: Vec<_> = points.iter().map(|p| p.y).collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(ys, vec![-1.0, -1.0, 1.0, 1.0]);
    }

    #[test]
    fn generate_returns_empty_for_non_positive_extent() {
        let strategy = JitterGridSampling::new(0.0, 1.0);
        let mut rng = StdRng::seed_from_u64(42);
        assert!(strategy
            .generate(Vec2::new(0.0, 5.0).into(), &mut rng)
            .is_empty());
        assert!(strategy
            .generate(Vec2::new(5.0, 0.0).into(), &mut rng)
            .is_empty());
    }
}
