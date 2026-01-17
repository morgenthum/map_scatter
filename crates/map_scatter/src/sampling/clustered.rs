//! Clustered position sampling (Thomas/Neyman–Scott processes).
use glam::Vec2;
use mint::Vector2;
use rand::RngCore;

use crate::sampling::{next_down, rand01, PositionSampling};

/// Strategy for placing parent (cluster center) points.
#[derive(Debug, Clone, Copy)]
pub enum ParentStrategy {
    /// Fixed number of parent centers.
    Count(
        /// Number of parent centers to generate.
        usize,
    ),
    /// Parent density per unit area.
    Density(
        /// Parents per unit area.
        f32,
    ),
}

/// Displacement kernel for children relative to their parent center.
#[derive(Debug, Clone, Copy)]
pub enum ClusterKernel {
    /// Thomas process: isotropic Gaussian with standard deviation `sigma`.
    Gaussian {
        /// Standard deviation of the Gaussian kernel.
        sigma: f32,
    },
    /// Neyman–Scott process: uniform within a disk of radius `radius`.
    UniformDisk {
        /// Disk radius for uniform sampling.
        radius: f32,
    },
}

/// Clustered sampling (Thomas/Neyman–Scott).
#[derive(Debug, Clone)]
pub struct ClusteredSampling {
    /// Parent placement strategy (fixed count or density).
    pub parents: ParentStrategy,
    /// Mean number of children per parent (Poisson-distributed).
    pub mean_children: f32,
    /// Child displacement kernel (Gaussian or uniform disk).
    pub kernel: ClusterKernel,
    /// If true, clamp results strictly inside right/top edges of the domain.
    pub clamp_inside: bool,
}

impl ClusteredSampling {
    /// Thomas process with fixed number of parents.
    pub fn thomas_with_count(parent_count: usize, mean_children: f32, sigma: f32) -> Self {
        Self {
            parents: ParentStrategy::Count(parent_count),
            mean_children,
            kernel: ClusterKernel::Gaussian { sigma },
            clamp_inside: true,
        }
    }

    /// Thomas process with parent density (parents per unit area).
    pub fn thomas_with_density(density: f32, mean_children: f32, sigma: f32) -> Self {
        Self {
            parents: ParentStrategy::Density(density),
            mean_children,
            kernel: ClusterKernel::Gaussian { sigma },
            clamp_inside: true,
        }
    }

    /// Neyman–Scott process with fixed number of parents.
    pub fn neyman_scott_with_count(parent_count: usize, mean_children: f32, radius: f32) -> Self {
        Self {
            parents: ParentStrategy::Count(parent_count),
            mean_children,
            kernel: ClusterKernel::UniformDisk { radius },
            clamp_inside: true,
        }
    }

    /// Neyman–Scott process with parent density (parents per unit area).
    pub fn neyman_scott_with_density(density: f32, mean_children: f32, radius: f32) -> Self {
        Self {
            parents: ParentStrategy::Density(density),
            mean_children,
            kernel: ClusterKernel::UniformDisk { radius },
            clamp_inside: true,
        }
    }

    /// Enable/disable clamping inside right/top edges (builder-style).
    pub fn with_clamp_inside(mut self, clamp: bool) -> Self {
        self.clamp_inside = clamp;
        self
    }
}

impl PositionSampling for ClusteredSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        let w = domain_extent.x;
        let h = domain_extent.y;

        if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }

        let half_w = w * 0.5;
        let half_h = h * 0.5;
        // Next representable floats below the right/top edges to enforce strict < comparisons
        let max_x = next_down(half_w);
        let max_y = next_down(half_h);

        // Determine number of parents
        let parent_count = match self.parents {
            ParentStrategy::Count(n) => n,
            ParentStrategy::Density(d) => {
                let lam = (d.max(0.0)) * (w * h);
                poisson_knuth(lam, rng) as usize
            }
        };

        if parent_count == 0 || self.mean_children <= 0.0 {
            return Vec::new();
        }

        // Estimate capacity: parents × mean_children (rounded up), but at least 1.
        let mut out =
            Vec::with_capacity(((parent_count as f32) * self.mean_children).ceil() as usize);

        // Generate parent positions uniformly in the domain
        for _ in 0..parent_count {
            let parent_x = -half_w + rand01(rng) * w;
            let parent_y = -half_h + rand01(rng) * h;
            let parent = Vec2::new(parent_x, parent_y);

            // Number of children for this parent
            let k = poisson_knuth(self.mean_children.max(0.0), rng) as usize;
            if k == 0 {
                continue;
            }

            // Generate children around the parent
            match self.kernel {
                ClusterKernel::Gaussian { sigma } => {
                    let s = sigma.max(0.0);
                    for _ in 0..k {
                        let (nx, ny) = box_muller_pair(rng);
                        let mut x = parent.x + s * nx;
                        let mut y = parent.y + s * ny;

                        if self.clamp_inside {
                            x = x.clamp(-half_w, max_x);
                            y = y.clamp(-half_h, max_y);
                        }

                        // If not clamping, still ensure finite values
                        if x.is_finite() && y.is_finite() {
                            out.push(Vec2::new(x, y));
                        }
                    }
                }
                ClusterKernel::UniformDisk { radius } => {
                    let r = radius.max(0.0);
                    for _ in 0..k {
                        // Uniform in disk via sqrt of radius and random angle
                        let ru = r * rand01(rng).sqrt();
                        let theta = 2.0 * core::f32::consts::PI * rand01(rng);
                        let mut x = parent.x + ru * theta.cos();
                        let mut y = parent.y + ru * theta.sin();

                        if self.clamp_inside {
                            x = x.clamp(-half_w, max_x);
                            y = y.clamp(-half_h, max_y);
                        }

                        if x.is_finite() && y.is_finite() {
                            out.push(Vec2::new(x, y));
                        }
                    }
                }
            }
        }

        out.into_iter().map(Into::into).collect()
    }
}

fn poisson_knuth(lambda: f32, rng: &mut dyn RngCore) -> u32 {
    if !(lambda.is_finite()) || lambda <= 0.0 {
        return 0;
    }

    let l = (-lambda).exp();
    let mut k: u32 = 0;
    let mut p: f32 = 1.0;

    loop {
        k += 1;
        p *= rand01(rng);
        if p <= l {
            return k - 1;
        }

        if k > 10_000_000 {
            return k - 1;
        }
    }
}

fn box_muller_pair(rng: &mut dyn RngCore) -> (f32, f32) {
    let u1 = (1.0 - rand01(rng)).clamp(f32::MIN_POSITIVE, 1.0);
    let u2 = rand01(rng);

    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * core::f32::consts::PI * u2;

    (r * theta.cos(), r * theta.sin())
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn empty_for_non_positive_extent_or_zero_parents_or_zero_children_mean() {
        let mut rng = StdRng::seed_from_u64(1);

        // Non-positive extent
        let s = ClusteredSampling::thomas_with_count(10, 3.0, 1.0);
        assert!(s.generate(Vec2::new(0.0, 10.0).into(), &mut rng).is_empty());
        assert!(s.generate(Vec2::new(10.0, 0.0).into(), &mut rng).is_empty());

        // Zero parents (fixed count)
        let s = ClusteredSampling::neyman_scott_with_count(0, 3.0, 2.0);
        assert!(s
            .generate(Vec2::new(10.0, 10.0).into(), &mut rng)
            .is_empty());

        // Zero mean children
        let s = ClusteredSampling::thomas_with_count(10, 0.0, 1.0);
        assert!(s
            .generate(Vec2::new(10.0, 10.0).into(), &mut rng)
            .is_empty());
    }

    #[test]
    fn results_are_within_bounds_and_deterministic_for_same_seed() {
        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(123);

        let s = ClusteredSampling::thomas_with_count(25, 2.0, 1.5).with_clamp_inside(true);

        let a = s.generate(Vec2::new(20.0, 10.0).into(), &mut rng_a);
        let b = s.generate(Vec2::new(20.0, 10.0).into(), &mut rng_b);

        // Determinism for same seed
        assert_eq!(a, b);

        // Bounds
        let half_w = 10.0;
        let half_h = 5.0;
        for p in a {
            assert!(p.x >= -half_w && p.x < half_w);
            assert!(p.y >= -half_h && p.y < half_h);
        }
    }

    #[test]
    fn neyman_scott_generates_points() {
        let mut rng = StdRng::seed_from_u64(999);
        let s = ClusteredSampling::neyman_scott_with_density(0.05, 5.0, 2.0);
        let pts = s.generate(Vec2::new(100.0, 50.0).into(), &mut rng);
        // With high probability we should see at least a few points.
        assert!(!pts.is_empty());
    }
}
