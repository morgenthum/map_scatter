//! Halton sequence position sampling strategy.
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Halton low-discrepancy sampling over a rectangular domain.
#[derive(Debug, Clone)]
pub struct HaltonSampling {
    /// Number of candidate points to generate.
    pub count: usize,
    /// Bases for the 2D Halton sequence. Defaults to `(2, 3)`.
    pub bases: (u32, u32),
    /// Starting index into the sequence.
    pub start_index: u32,
    /// If true, apply Cranley–Patterson rotation with random offsets from the RNG.
    pub rotate: bool,
}

impl HaltonSampling {
    /// Construct a Halton sampler with default bases (2, 3), start_index = 1, no rotation.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            bases: (2, 3),
            start_index: 1,
            rotate: false,
        }
    }

    /// Construct with rotation flag (CP rotation), using default bases (2, 3) and start_index = 1.
    pub fn with_rotation(count: usize, rotate: bool) -> Self {
        Self {
            count,
            bases: (2, 3),
            start_index: 1,
            rotate,
        }
    }

    /// Construct with custom bases and rotation flag; start_index defaults to 1.
    ///
    /// Panics if either base is less than 2.
    pub fn with_bases(count: usize, bases: (u32, u32), rotate: bool) -> Self {
        assert!(bases.0 >= 2 && bases.1 >= 2, "Halton bases must be >= 2");
        Self {
            count,
            bases,
            start_index: 1,
            rotate,
        }
    }

    /// Set the starting index (builder-style).
    pub fn with_start_index(mut self, start_index: u32) -> Self {
        self.start_index = start_index;
        self
    }
}

impl PositionSampling for HaltonSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let w = domain_extent.x;
        let h = domain_extent.y;

        if self.count == 0 || w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }
        let (b1, b2) = self.bases;
        debug_assert!(b1 >= 2 && b2 >= 2);

        // Cranley–Patterson rotation offsets in [0,1] if enabled.
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
        let start = self.start_index as u64;

        for i in 0..self.count {
            let idx = start + i as u64;

            let mut u = radical_inverse(idx, b1);
            let mut v = radical_inverse(idx, b2);

            // Apply CP rotation: add offsets, wrap to [0,1]
            u = frac(u + dx);
            v = frac(v + dy);

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

#[inline]
fn frac(x: f32) -> f32 {
    x - x.floor()
}

/// Compute the radical inverse of `n` in the given `base`.
fn radical_inverse(mut n: u64, base: u32) -> f32 {
    debug_assert!(base >= 2);
    let b = base as f32;
    let inv_b = 1.0 / b;

    if n == 0 {
        return 0.0;
    }

    let mut f = inv_b;
    let mut result = 0.0_f32;

    while n > 0 {
        let digit = (n % base as u64) as f32;
        result += digit * f;
        n /= base as u64;
        f *= inv_b;
    }

    // result is already in [0,1]; guard numerical noise
    if result >= 1.0 {
        next_down(1.0)
    } else {
        result
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
        let s0 = HaltonSampling::new(0);
        assert!(s0
            .generate(Vec2::new(10.0, 10.0).into(), &mut rng)
            .is_empty());

        let s1 = HaltonSampling::new(10);
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
    fn bounds_and_count_respected() {
        let mut rng = StdRng::seed_from_u64(42);
        let s = HaltonSampling::new(128);
        let pts = s.generate(Vec2::new(9.0, 5.0).into(), &mut rng);
        assert_eq!(pts.len(), 128);

        let half_w = 4.5;
        let half_h = 2.5;
        for p in pts {
            assert!(p.x >= -half_w && p.x < half_w);
            assert!(p.y >= -half_h && p.y < half_h);
        }
    }

    #[test]
    fn determinism_without_rotation() {
        let s = HaltonSampling::new(64).with_start_index(1);

        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(987);

        // No rotation -> RNG does not impact the sequence
        let pa = s.generate(Vec2::new(10.0, 10.0).into(), &mut rng_a);
        let pb = s.generate(Vec2::new(10.0, 10.0).into(), &mut rng_b);
        assert_eq!(pa, pb);
    }

    #[test]
    fn rotation_changes_distribution() {
        let s_rot = HaltonSampling::with_rotation(64, true).with_start_index(1);

        let mut rng_c = StdRng::seed_from_u64(123);
        let mut rng_d = StdRng::seed_from_u64(987);

        let pc = s_rot.generate(Vec2::new(10.0, 10.0).into(), &mut rng_c);
        let pd = s_rot.generate(Vec2::new(10.0, 10.0).into(), &mut rng_d);
        assert_ne!(pc, pd);
    }

    #[test]
    fn radical_inverse_basic() {
        // Base-2: n=1 -> 0.1b = 0.5; n=2 -> 0.01b = 0.25; n=3 -> 0.11b = 0.75
        assert!((radical_inverse(1, 2) - 0.5).abs() < 1e-6);
        assert!((radical_inverse(2, 2) - 0.25).abs() < 1e-6);
        assert!((radical_inverse(3, 2) - 0.75).abs() < 1e-6);

        // Base-3: n=1 -> 1/3; n=2 -> 2/3; n=3 -> 1/9
        assert!((radical_inverse(1, 3) - (1.0 / 3.0)).abs() < 1e-6);
        assert!((radical_inverse(2, 3) - (2.0 / 3.0)).abs() < 1e-6);
        assert!((radical_inverse(3, 3) - (1.0 / 9.0)).abs() < 1e-6);
    }
}
