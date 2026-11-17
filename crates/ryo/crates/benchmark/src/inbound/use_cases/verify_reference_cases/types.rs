//! What a reference-only verification answers.
use crate::outbound::evidence::types::RuleTiming;
use ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount;
use std::collections::BTreeMap;

/// The cases a reference-only verification matched and how long each rule's cases took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedReferenceCases {
    /// How many cases matched the reference.
    pub count: ReferenceCaseCount,
    /// How long each rule's cases took.
    pub timings: BTreeMap<ryo::linter::RuleId, RuleTiming>,
}
