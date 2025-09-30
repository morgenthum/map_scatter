use std::time::Duration;

use criterion::{Criterion, Throughput};

pub const SAMPLE_SIZE: usize = 20;
pub const WARM_UP: Duration = Duration::from_secs(1);
pub const MEASUREMENT_TIME: Duration = Duration::from_secs(2);

pub fn default_criterion() -> Criterion {
    Criterion::default()
        .configure_from_args()
        .sample_size(SAMPLE_SIZE)
        .warm_up_time(WARM_UP)
        .measurement_time(MEASUREMENT_TIME)
}

pub fn elements_throughput(elements: usize) -> Throughput {
    Throughput::Elements(elements.max(1) as u64)
}
