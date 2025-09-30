//! Selection utilities for choosing a [crate::scatter::Kind] from evaluation results.
//!
//! This module provides helpers to pick a kind after evaluating candidates:
//! - [pick_weighted_random]: draws proportionally to each allowed kind's weight in [crate::scatter::evaluator::KindEvaluation].
//! - [pick_highest_probability]: picks the allowed kind with the maximum weight in [crate::scatter::evaluator::KindEvaluation].
//!
//! Inputs are slices of [crate::scatter::evaluator::KindEvaluation] produced by
//! evaluators such as [crate::scatter::evaluator::Evaluator] or during plan execution
//! in [crate::scatter::runner]. When randomness is required, pass an RNG that
//! implements [rand::RngCore].
//!
//! Related modules: [crate::scatter::plan] (selection configured via
//! [crate::scatter::plan::SelectionStrategy]) and [crate::sampling] (candidate generation).
use rand::RngCore;

use crate::scatter::evaluator::KindEvaluation;
use crate::scatter::Kind;

pub fn pick_weighted_random<R: RngCore>(results: &[KindEvaluation], rng: &mut R) -> Option<Kind> {
    let placeable: Vec<_> = results.iter().filter(|r| r.allowed).collect();
    if placeable.is_empty() {
        return None;
    }

    let total_weight: f32 = placeable.iter().map(|r| r.weight).sum();
    if total_weight <= 0.0 {
        return None;
    }

    let mut roll = crate::sampling::rand01(rng) * total_weight;
    for r in &placeable {
        roll -= r.weight;
        if roll <= 0.0 {
            return Some(r.kind.clone());
        }
    }

    placeable.first().map(|r| r.kind.clone())
}

pub fn pick_highest_probability(results: &[KindEvaluation]) -> Option<Kind> {
    results
        .iter()
        .filter(|r| r.allowed)
        .max_by(|a, b| a.weight.total_cmp(&b.weight))
        .map(|r| r.kind.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fieldgraph::spec::FieldGraphSpec;

    fn kind(id: &str) -> Kind {
        Kind::new(id, FieldGraphSpec::default())
    }

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
    fn weighted_random_selects_by_probability() {
        let results = vec![
            KindEvaluation {
                kind: kind("a"),
                allowed: true,
                weight: 0.7,
            },
            KindEvaluation {
                kind: kind("b"),
                allowed: true,
                weight: 0.3,
            },
        ];

        let mut rng_first = FixedRng { value: 0 }; // select first
        assert_eq!(
            pick_weighted_random(&results, &mut rng_first).unwrap().id,
            "a"
        );

        let mut rng_second = FixedRng {
            value: (0.8 * u32::MAX as f32) as u32,
        };
        assert_eq!(
            pick_weighted_random(&results, &mut rng_second).unwrap().id,
            "b"
        );
    }

    #[test]
    fn weighted_random_returns_none_when_disallowed() {
        let results = vec![KindEvaluation {
            kind: kind("a"),
            allowed: false,
            weight: 1.0,
        }];
        let mut rng = FixedRng { value: 0 };
        assert!(pick_weighted_random(&results, &mut rng).is_none());
    }

    #[test]
    fn highest_probability_picks_max_allowed() {
        let results = vec![
            KindEvaluation {
                kind: kind("a"),
                allowed: true,
                weight: 0.2,
            },
            KindEvaluation {
                kind: kind("b"),
                allowed: true,
                weight: 0.8,
            },
        ];
        assert_eq!(pick_highest_probability(&results).unwrap().id, "b");
    }

    #[test]
    fn highest_probability_returns_none_when_all_blocked() {
        let results = vec![KindEvaluation {
            kind: kind("a"),
            allowed: false,
            weight: 1.0,
        }];
        assert!(pick_highest_probability(&results).is_none());
    }
}
