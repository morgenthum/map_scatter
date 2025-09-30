//! Best-candidate (Mitchell’s) position sampling strategy.
use glam::Vec2;
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Best-candidate (Mitchell's) sampling over a rectangular domain.
#[derive(Debug, Clone)]
pub struct BestCandidateSampling {
    /// Number of candidate points to generate.
    pub count: usize,
    /// Number of random trials per point. Higher `k` => better blue-noise at higher cost.
    pub k: usize,
}

impl BestCandidateSampling {
    /// Create a new best-candidate sampler with a target `count` and `k` trials per point.
    pub fn new(count: usize, k: usize) -> Self {
        Self { count, k: k.max(1) }
    }
}

impl PositionSampling for BestCandidateSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let domain_extent = Vec2::from(domain_extent);
        let w = domain_extent.x;
        let h = domain_extent.y;

        if self.count == 0 || w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }

        let half_w = w * 0.5;
        let half_h = h * 0.5;
        // Next representable floats below +half_w/+half_h to enforce strict < bound
        let max_x = next_down(half_w);
        let max_y = next_down(half_h);

        let mut points: Vec<Vec2> = Vec::with_capacity(self.count);

        for _ in 0..self.count {
            // If there are no points yet, just pick a random one
            if points.is_empty() {
                let u = rand01(rng);
                let v = rand01(rng);
                let mut x = u * w - half_w;
                let mut y = v * h - half_h;
                x = x.clamp(-half_w, max_x);
                y = y.clamp(-half_h, max_y);
                points.push(Vec2::new(x, y));
                continue;
            }

            // Draw k candidates and select the one farthest from the existing set
            let mut best_candidate: Option<Vec2> = None;
            let mut best_d2 = -1.0_f32;

            for _ in 0..self.k {
                let u = rand01(rng);
                let v = rand01(rng);
                let mut x = u * w - half_w;
                let mut y = v * h - half_h;
                x = x.clamp(-half_w, max_x);
                y = y.clamp(-half_h, max_y);

                let p = Vec2::new(x, y);
                let d2 = {
                    if points.is_empty() {
                        f32::INFINITY
                    } else {
                        let mut best = f32::INFINITY;
                        for &q in &points {
                            let d = p - q;
                            let dsq = d.x * d.x + d.y * d.y;
                            if dsq < best {
                                best = dsq;
                            }
                        }
                        best
                    }
                };

                if d2 > best_d2 {
                    best_d2 = d2;
                    best_candidate = Some(p);
                }
            }

            if let Some(p) = best_candidate {
                points.push(p);
            } else {
                // Fallback (should not happen with k >= 1)
                let u = rand01(rng);
                let v = rand01(rng);
                let mut x = u * w - half_w;
                let mut y = v * h - half_h;
                x = x.clamp(-half_w, max_x);
                y = y.clamp(-half_h, max_y);
                points.push(Vec2::new(x, y));
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
    fn empty_for_zero_count_or_non_positive_extent() {
        let mut rng = StdRng::seed_from_u64(1);

        let s0 = BestCandidateSampling::new(0, 16);
        assert!(s0
            .generate(Vec2::new(10.0, 10.0).into(), &mut rng)
            .is_empty());

        let s1 = BestCandidateSampling::new(10, 16);
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
        let s = BestCandidateSampling::new(128, 16);
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
    fn determinism_for_same_seed() {
        let s = BestCandidateSampling::new(64, 8);

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
