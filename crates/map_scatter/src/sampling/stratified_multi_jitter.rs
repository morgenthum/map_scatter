//! Stratified multi-jittered position sampling.
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Stratified multi-jittered (CMJ-style) position sampling.
#[derive(Debug, Clone)]
pub struct StratifiedMultiJitterSampling {
    /// Number of candidate points to generate.
    pub count: usize,
    /// Apply Cranley–Patterson rotation with random offsets from the RNG.
    pub rotate: bool,
}

impl StratifiedMultiJitterSampling {
    /// Construct a multi-jittered sampler with `count` points and no CP rotation.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            rotate: false,
        }
    }

    /// Construct with optional CP rotation (global random offset in `[0,1]^2`).
    pub fn with_rotation(count: usize, rotate: bool) -> Self {
        Self { count, rotate }
    }
}

impl PositionSampling for StratifiedMultiJitterSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let w = domain_extent.x;
        let h = domain_extent.y;

        if self.count == 0 || w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }

        // Choose a near-square grid (nx, ny) that can accommodate `count` samples.
        let nx = (self.count as f32).sqrt().ceil() as usize;
        let ny = self.count.div_ceil(nx).max(1);

        // CP rotation offsets in [0,1] if enabled
        let (dx, dy) = if self.rotate {
            (rand01(rng), rand01(rng))
        } else {
            (0.0, 0.0)
        };

        // Precompute per-row and per-column permutations for correlated jittering
        // - col_perm_per_row[j][i] permutes column index i within row j
        // - row_perm_per_col[i][j] permutes row index j within column i
        let mut col_perm_per_row: Vec<Vec<usize>> = Vec::with_capacity(ny);
        for _ in 0..ny {
            let mut v: Vec<usize> = (0..nx).collect();
            fisher_yates_shuffle(&mut v, rng);
            col_perm_per_row.push(v);
        }
        let mut row_perm_per_col: Vec<Vec<usize>> = Vec::with_capacity(nx);
        for _ in 0..nx {
            let mut v: Vec<usize> = (0..ny).collect();
            fisher_yates_shuffle(&mut v, rng);
            row_perm_per_col.push(v);
        }

        let half_w = w * 0.5;
        let half_h = h * 0.5;
        // Next representable floats below the right/top edges to enforce strict < comparisons
        let max_x = next_down(half_w);
        let max_y = next_down(half_h);

        let mut out = Vec::with_capacity(self.count);

        for s in 0..self.count {
            let i = s % nx; // column index
            let j = s / nx; // row index
            if j >= ny {
                break;
            }

            // Correlated multi-jitter:
            // - Permute the opposing axis index for each stratum.
            // - Add within-cell jitter.
            let sx = col_perm_per_row[j][i]; // permuted column for row j
            let sy = row_perm_per_col[i][j]; // permuted row for column i
            let jx = rand01(rng);
            let jy = rand01(rng);

            // Stratified positions in [0,1]
            let mut u = (i as f32 + ((sy as f32 + jx) / ny as f32)) / nx as f32;
            let mut v = (j as f32 + ((sx as f32 + jy) / nx as f32)) / ny as f32;

            // CP rotation
            u = frac(u + dx);
            v = frac(v + dy);

            // Map to origin-centered rectangle
            let mut x = u * w - half_w;
            let mut y = v * h - half_h;

            // Keep strictly inside right/top edges
            x = x.clamp(-half_w, max_x);
            y = y.clamp(-half_h, max_y);

            out.push(Vector2 { x, y });
        }

        out
    }
}

#[inline]
fn frac(x: f32) -> f32 {
    x - x.floor()
}

/// In-place Fisher–Yates shuffle using the provided RNG.
fn fisher_yates_shuffle<T>(arr: &mut [T], rng: &mut dyn RngCore) {
    let mut n = arr.len();
    while n > 1 {
        // Choose a random index in [0, n)
        let k = (rng.next_u32() as usize) % n;
        n -= 1;
        arr.swap(n, k);
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn empty_for_zero_count_or_non_positive_extent() {
        let mut rng = StdRng::seed_from_u64(1);
        let s0 = StratifiedMultiJitterSampling::new(0);
        assert!(s0
            .generate(Vec2::new(10.0, 10.0).into(), &mut rng)
            .is_empty());

        let s1 = StratifiedMultiJitterSampling::new(10);
        assert!(s1
            .generate(Vec2::new(0.0, 10.0).into(), &mut rng)
            .is_empty());
        assert!(s1
            .generate(Vec2::new(10.0, 0.0).into(), &mut rng)
            .is_empty());
        assert!(s1
            .generate(Vec2::new(-5.0, 2.0).into(), &mut rng)
            .is_empty());
    }

    #[test]
    fn count_and_bounds_are_respected() {
        let mut rng = StdRng::seed_from_u64(42);
        let s = StratifiedMultiJitterSampling::with_rotation(200, false);
        let pts = s.generate(Vec2::new(13.0, 7.0).into(), &mut rng);
        assert_eq!(pts.len(), 200);

        let half_w = 6.5;
        let half_h = 3.5;
        for p in pts {
            assert!(p.x >= -half_w && p.x < half_w);
            assert!(p.y >= -half_h && p.y < half_h);
        }
    }

    #[test]
    fn determinism_for_same_seed() {
        let s = StratifiedMultiJitterSampling::with_rotation(128, true);

        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(123);
        let pa = s.generate(Vec2::new(10.0, 10.0).into(), &mut rng_a);
        let pb = s.generate(Vec2::new(10.0, 10.0).into(), &mut rng_b);
        assert_eq!(pa, pb);

        let mut rng_c = StdRng::seed_from_u64(456);
        let pc = s.generate(Vec2::new(10.0, 10.0).into(), &mut rng_c);
        if s.count > 0 {
            assert_ne!(pa, pc);
        }
    }
}
