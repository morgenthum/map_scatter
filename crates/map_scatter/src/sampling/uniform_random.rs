//! Uniform random position sampling strategy.
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Uniform i.i.d. random sampling over a rectangular domain.
#[derive(Debug, Clone)]
pub struct UniformRandomSampling {
    /// Number of candidate points to generate.
    pub count: usize,
}

impl UniformRandomSampling {
    /// Create a new uniform random sampler that generates `count` points.
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

impl PositionSampling for UniformRandomSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let w = domain_extent.x;
        let h = domain_extent.y;

        if self.count == 0 || w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }

        let half_w = w * 0.5;
        let half_h = h * 0.5;
        // Next representable floats below the right/top edges to enforce strict < comparisons
        let max_x = next_down(half_w);
        let max_y = next_down(half_h);

        let mut out = Vec::with_capacity(self.count);
        for _ in 0..self.count {
            let u = rand01(rng);
            let v = rand01(rng);

            let mut x = u * w - half_w;
            let mut y = v * h - half_h;

            // Keep strictly inside right/top edges.
            x = x.clamp(-half_w, max_x);
            y = y.clamp(-half_h, max_y);

            out.push(Vector2 { x, y });
        }

        out
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

        let s0 = UniformRandomSampling::new(0);
        assert!(s0
            .generate(Vec2::new(10.0, 10.0).into(), &mut rng)
            .is_empty());

        let s1 = UniformRandomSampling::new(10);
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
        let s = UniformRandomSampling::new(100);
        let pts = s.generate(Vec2::new(8.0, 6.0).into(), &mut rng);
        assert_eq!(pts.len(), 100);

        let half_w = 4.0;
        let half_h = 3.0;
        for p in pts {
            assert!(p.x >= -half_w && p.x < half_w);
            assert!(p.y >= -half_h && p.y < half_h);
        }
    }

    #[test]
    fn determinism_for_same_seed() {
        let s = UniformRandomSampling::new(32);

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
