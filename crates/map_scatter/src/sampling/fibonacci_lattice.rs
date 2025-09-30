//! Fibonacci lattice position sampling strategy.
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Fibonacci lattice position sampling.
#[derive(Debug, Clone)]
pub struct FibonacciLatticeSampling {
    /// Number of candidate points to generate.
    pub count: usize,
    /// If true, apply Cranley–Patterson rotation with random offsets from the RNG.
    pub rotate: bool,
}

impl FibonacciLatticeSampling {
    /// Create a new Fibonacci lattice sampler with a fixed `count` of points and no rotation.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            rotate: false,
        }
    }

    /// Create a new Fibonacci lattice sampler with `count` and optional rotation.
    pub fn with_rotation(count: usize, rotate: bool) -> Self {
        Self { count, rotate }
    }
}

impl PositionSampling for FibonacciLatticeSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let w = domain_extent.x;
        let h = domain_extent.y;

        if self.count == 0 || w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }

        const PHI: f32 = 1.618_034_f32; // (1 + sqrt(5)) / 2
        let alpha = 1.0 / PHI;

        let (dx, dy) = if self.rotate {
            (rand01(rng), rand01(rng))
        } else {
            (0.0, 0.0)
        };

        let half_w = w * 0.5;
        let half_h = h * 0.5;

        let max_x = next_down(half_w);
        let max_y = next_down(half_h);

        let mut out = Vec::with_capacity(self.count);

        for i in 0..self.count {
            let fi = i as f32;

            // Fibonacci lattice: x evenly spaced with offset, y from Kronecker sequence
            let u = (fi + dx) / self.count as f32;
            let v = frac(fi * alpha + dy);

            // Map to origin-centered rectangle; keep inside bounds.
            let mut x = u * w - half_w;
            let mut y = v * h - half_h;

            // Keep strictly inside right/top edges to match convention elsewhere.
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

#[cfg(test)]
mod tests {
    use glam::Vec2;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn generate_empty_for_zero_count_or_non_positive_extent() {
        let mut rng = StdRng::seed_from_u64(1);
        let s = FibonacciLatticeSampling::new(0);
        assert!(s
            .generate(Vec2::new(10.0, 10.0).into(), &mut rng)
            .is_empty());

        let s = FibonacciLatticeSampling::new(10);
        assert!(s.generate(Vec2::new(0.0, 10.0).into(), &mut rng).is_empty());
        assert!(s.generate(Vec2::new(10.0, 0.0).into(), &mut rng).is_empty());
        assert!(s.generate(Vec2::new(-5.0, 2.0).into(), &mut rng).is_empty());
    }

    #[test]
    fn points_are_within_domain() {
        let mut rng = StdRng::seed_from_u64(42);
        let s = FibonacciLatticeSampling::new(100);
        let pts = s.generate(Vec2::new(7.0, 3.0).into(), &mut rng);
        assert_eq!(pts.len(), 100);

        let half_w = 3.5;
        let half_h = 1.5;

        for p in pts {
            assert!(p.x >= -half_w && p.x < half_w);
            assert!(p.y >= -half_h && p.y < half_h);
        }
    }

    #[test]
    fn rotation_changes_distribution() {
        // Without rotation, two different RNG seeds should not matter (sequence is deterministic).
        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(987);
        let s_no_rot = FibonacciLatticeSampling::with_rotation(16, false);
        let pa = s_no_rot.generate(Vec2::new(10.0, 10.0).into(), &mut rng_a);
        let pb = s_no_rot.generate(Vec2::new(10.0, 10.0).into(), &mut rng_b);
        assert_eq!(pa, pb);

        // With rotation, different seeds should produce different sequences with high probability.
        let mut rng_c = StdRng::seed_from_u64(123);
        let mut rng_d = StdRng::seed_from_u64(987);
        let s_rot = FibonacciLatticeSampling::with_rotation(16, true);
        let pc = s_rot.generate(Vec2::new(10.0, 10.0).into(), &mut rng_c);
        let pd = s_rot.generate(Vec2::new(10.0, 10.0).into(), &mut rng_d);
        assert_ne!(pc, pd);
    }
}
