//! A real downstream refusal interrupts partition construction and releases its backing.
use super::fixtures::{Refusal, exercise};

#[test]
fn later_partition_refusal_stops_work_and_releases_backing() {
    for refusal in [
        Refusal::CollectedPattern,
        Refusal::DeliveredPattern,
        Refusal::Callback,
    ] {
        for fused in [false, true] {
            exercise(refusal, fused);
        }
    }
}
