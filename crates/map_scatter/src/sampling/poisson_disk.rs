//! Poisson disk position sampling strategy.
use std::collections::VecDeque;
use std::f32::consts::PI;

use glam::Vec2;
use mint::Vector2;
use rand::RngCore;

use crate::sampling::PositionSampling;

/// Poisson disk sampling strategy.
#[derive(Debug, Clone)]
pub struct PoissonDiskSampling {
    /// Minimum distance between samples in world units.
    pub radius: f32,
}

impl PositionSampling for PoissonDiskSampling {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>> {
        if !self.radius.is_finite() || self.radius <= 0.0 {
            return Vec::new();
        }

        let mut sampler = PoissonDiskSampler::new(self.radius, Vec2::from(domain_extent));
        sampler.generate(rng).into_iter().map(Into::into).collect()
    }
}

impl PoissonDiskSampling {
    /// Create a new PoissonDiskSampling with specified radius.
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

struct PoissonDiskSampler {
    radius: f32,
    radius_squared: f32,
    cell_size: f32,
    grid_width: usize,
    grid_height: usize,
    grid: Vec<Option<Vec2>>,
    active_list: VecDeque<Vec2>,
    bounds: Vec2,
}

impl PoissonDiskSampler {
    pub fn new(radius: f32, bounds: Vec2) -> Self {
        debug_assert!(radius > 0.0);
        let radius_squared = radius * radius;
        let cell_size = radius / std::f32::consts::SQRT_2;
        let grid_width = (bounds.x / cell_size).ceil() as usize + 1;
        let grid_height = (bounds.y / cell_size).ceil() as usize + 1;

        Self {
            radius,
            radius_squared,
            cell_size,
            grid_width,
            grid_height,
            grid: vec![None; grid_width * grid_height],
            active_list: VecDeque::new(),
            bounds,
        }
    }

    #[inline]
    fn grid_index(&self, x: usize, y: usize) -> usize {
        y * self.grid_width + x
    }

    #[inline]
    fn point_to_grid(&self, point: Vec2) -> (usize, usize) {
        let centered_x = point.x + self.bounds.x / 2.0;
        let centered_y = point.y + self.bounds.y / 2.0;
        let x = ((centered_x / self.cell_size).floor() as isize)
            .clamp(0, self.grid_width as isize - 1) as usize;
        let y = ((centered_y / self.cell_size).floor() as isize)
            .clamp(0, self.grid_height as isize - 1) as usize;
        (x, y)
    }

    fn is_valid_point(&self, point: Vec2) -> bool {
        let half_x = self.bounds.x / 2.0;
        let half_y = self.bounds.y / 2.0;
        if point.x < -half_x || point.x >= half_x || point.y < -half_y || point.y >= half_y {
            return false;
        }

        let (gx, gy) = self.point_to_grid(point);
        let start_x = gx.saturating_sub(2);
        let end_x = (gx + 3).min(self.grid_width);
        let start_y = gy.saturating_sub(2);
        let end_y = (gy + 3).min(self.grid_height);

        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = self.grid_index(x, y);
                if let Some(existing) = self.grid[idx] {
                    let dx = point.x - existing.x;
                    let dy = point.y - existing.y;
                    let dist2 = dx * dx + dy * dy;
                    if dist2 < self.radius_squared {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn add_point(&mut self, point: Vec2) {
        let (gx, gy) = self.point_to_grid(point);
        let idx = self.grid_index(gx, gy);
        self.grid[idx] = Some(point);
        self.active_list.push_back(point);
    }

    fn generate_around_point(&mut self, rng: &mut dyn RngCore, point: Vec2) -> Option<Vec2> {
        const MAX_ATTEMPTS: usize = 30;

        for _ in 0..MAX_ATTEMPTS {
            let angle = crate::sampling::rand01(rng) * 2.0 * PI;
            let distance = self.radius + crate::sampling::rand01(rng) * self.radius;

            let candidate = Vec2::new(
                point.x + angle.cos() * distance,
                point.y + angle.sin() * distance,
            );

            if self.is_valid_point(candidate) {
                return Some(candidate);
            }
        }

        None
    }

    pub fn generate(&mut self, rng: &mut dyn RngCore) -> Vec<Vec2> {
        let half_x = self.bounds.x / 2.0;
        let half_y = self.bounds.y / 2.0;

        let initial = Vec2::new(
            -half_x + crate::sampling::rand01(rng) * (2.0 * half_x),
            -half_y + crate::sampling::rand01(rng) * (2.0 * half_y),
        );
        self.add_point(initial);

        let mut points = vec![initial];

        while let Some(active) = self.active_list.pop_front() {
            let mut found_any = false;

            for _ in 0..5 {
                if let Some(p) = self.generate_around_point(rng, active) {
                    self.add_point(p);
                    points.push(p);
                    found_any = true;
                }
            }

            if found_any {
                self.active_list.push_back(active);
            }
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    fn pairwise_min_distance(points: &[mint::Vector2<f32>]) -> f32 {
        let mut min = f32::MAX;
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let a = glam::Vec2::from(points[i]);
                let b = glam::Vec2::from(points[j]);
                let dist = (a - b).length();
                if dist < min {
                    min = dist;
                }
            }
        }
        min
    }

    #[test]
    fn sampler_initializes_grid_dimensions() {
        let sampler = PoissonDiskSampler::new(0.5, Vec2::new(2.0, 1.0));
        assert_eq!(
            sampler.grid_width,
            ((2.0 / sampler.cell_size).ceil() as usize) + 1
        );
        assert_eq!(
            sampler.grid_height,
            ((1.0 / sampler.cell_size).ceil() as usize) + 1
        );
    }

    #[test]
    fn is_valid_point_rejects_close_neighbors() {
        let mut sampler = PoissonDiskSampler::new(1.0, Vec2::new(4.0, 4.0));
        let origin = Vec2::ZERO;
        sampler.add_point(origin);

        assert!(!sampler.is_valid_point(Vec2::new(0.5, 0.0)));
        assert!(sampler.is_valid_point(Vec2::new(1.5, 1.5)));
    }

    #[test]
    fn generated_points_respect_radius_constraint() {
        let mut rng = StdRng::seed_from_u64(123);
        let sampling = PoissonDiskSampling::new(0.3);
        let points = sampling.generate(Vec2::new(1.0, 1.0).into(), &mut rng);

        assert!(!points.is_empty());
        for p in &points {
            assert!(p.x >= -0.5 && p.x < 0.5);
            assert!(p.y >= -0.5 && p.y < 0.5);
        }
        if points.len() > 1 {
            assert!(pairwise_min_distance(&points) >= 0.3 - 1e-6);
        }
    }

    #[test]
    fn zero_radius_returns_no_points() {
        let mut rng = StdRng::seed_from_u64(1);
        let sampling = PoissonDiskSampling::new(0.0);
        let points = sampling.generate(Vec2::new(1.0, 1.0).into(), &mut rng);
        assert!(points.is_empty());
    }
}
