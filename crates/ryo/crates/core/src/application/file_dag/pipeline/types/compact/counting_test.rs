//! A selected compact cut keeps ordinary per-step work counts under a counting run.
use crate::domain::lowering::types::Counting;

#[test]
fn selected_compact_inputs_preserve_fed_work_with_counting_enabled() {
    super::subject_with_counting(false, Counting::Counts)
        .expect("a real selected cut retains output, refusal, and exact fed-row parity");
}
