//! What a verification answers.
use crate::outbound::evidence::types::RuleTiming;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::collections::BTreeMap;

/// The cases a verification checked and how long each rule's cases took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedCases {
    /// How many cases were checked.
    pub count: CaseCount,
    /// How long each rule's cases took.
    pub timings: BTreeMap<ryo::linter::RuleId, RuleTiming>,
}
