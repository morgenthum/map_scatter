//! Sampling strategies for generating candidate positions in a 2D domain.
//!
//! This module defines traits and concrete strategies used by the scatter pipeline
//! to propose positions prior to evaluation.
use mint::Vector2;
use rand::RngCore;

pub mod best_candidate;
pub mod clustered;
pub mod fibonacci_lattice;
pub mod halton;
pub mod hex_jitter_grid;
pub mod jitter_grid;
pub mod poisson_disk;
pub mod stratified_multi_jitter;
pub mod uniform_random;

pub use best_candidate::BestCandidateSampling;
pub use clustered::ClusteredSampling;
pub use fibonacci_lattice::FibonacciLatticeSampling;
pub use halton::HaltonSampling;
pub use hex_jitter_grid::HexJitterGridSampling;
pub use jitter_grid::JitterGridSampling;
pub use poisson_disk::PoissonDiskSampling;
pub use stratified_multi_jitter::StratifiedMultiJitterSampling;
pub use uniform_random::UniformRandomSampling;

/// Trait for position sampling.
pub trait PositionSampling: Send + Sync {
    fn generate(&self, domain_extent: Vector2<f32>, rng: &mut dyn RngCore) -> Vec<Vector2<f32>>;
}

/// Generate a random float in the range [0, 1].
#[inline]
pub(crate) fn rand01(rng: &mut dyn RngCore) -> f32 {
    (rng.next_u32() as f32) / ((u32::MAX as f32) + 1.0)
}

/// Compute the next smaller representable float value.
///
/// Returns a value that is strictly less than the input, useful for
/// ensuring bounds are strictly inside a domain. Handles edge cases
/// safely including very small positive values and zero.
#[inline]
pub(crate) fn next_down(val: f32) -> f32 {
    if val.is_nan() {
        return f32::NAN;
    }

    if val == f32::NEG_INFINITY {
        return f32::NEG_INFINITY;
    }

    if val == f32::INFINITY {
        return f32::MAX;
    }

    if val == 0.0 {
        return -f32::MIN_POSITIVE;
    }

    let bits = val.to_bits();
    if val > 0.0 {
        f32::from_bits(bits.saturating_sub(1))
    } else {
        f32::from_bits(bits.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedRng {
        value: u32,
    }

    impl RngCore for FixedRng {
        fn next_u32(&mut self) -> u32 {
            self.value
        }

        fn next_u64(&mut self) -> u64 {
            self.value as u64
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            let bytes = self.value.to_le_bytes();
            for (i, b) in dest.iter_mut().enumerate() {
                *b = bytes[i % 4];
            }
        }
    }

    #[test]
    fn rand01_returns_zero_for_zero_input() {
        let mut rng = FixedRng { value: 0 };
        let result = rand01(&mut rng);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn rand01_handles_max_value_correctly() {
        let mut rng = FixedRng { value: u32::MAX };
        let result = rand01(&mut rng);
        // Should return exactly 1.0 when input is u32::MAX
        assert_eq!(result, u32::MAX as f32 / (u32::MAX as f32 + 1.0));
        // Verify it's in range [0,1]
        assert!((0.0..=1.0).contains(&result));
        // Should be very close to 1.0 but not exceed it
        assert!(result < 1.0 || (result - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rand01_values_in_range() {
        // Test various values to ensure all are in [0,1]
        let test_values = vec![0, 1, 100, 1000, u32::MAX / 2, u32::MAX - 1, u32::MAX];

        for value in test_values {
            let mut rng = FixedRng { value };
            let result = rand01(&mut rng);
            assert!(
                (0.0..=1.0).contains(&result),
                "rand01({}) = {} is out of range [0,1]",
                value,
                result
            );
        }
    }

    #[test]
    fn next_down_handles_edge_cases() {
        // Normal positive values
        assert!(next_down(1.0) < 1.0);
        assert!(next_down(0.5) < 0.5);

        // Very small positive value retains positivity but shrinks
        let down_min_pos = next_down(f32::MIN_POSITIVE);
        assert!(down_min_pos >= 0.0);
        assert!(down_min_pos < f32::MIN_POSITIVE);

        // Zero and negative values
        assert_eq!(next_down(0.0), -f32::MIN_POSITIVE);
        assert!(next_down(-1.0) < -1.0);
        assert!(next_down(-100.0) < -100.0);

        // Non-finite values
        assert_eq!(next_down(f32::INFINITY), f32::MAX);
        assert_eq!(next_down(f32::NEG_INFINITY), f32::NEG_INFINITY);
        assert!(next_down(f32::NAN).is_nan());
    }

    #[test]
    fn rand01_distribution_properties() {
        let mut rng = FixedRng {
            value: u32::MAX / 2,
        };
        let result = rand01(&mut rng);
        // Should be approximately 0.5
        assert!((result - 0.5).abs() < 0.001);
    }
}
