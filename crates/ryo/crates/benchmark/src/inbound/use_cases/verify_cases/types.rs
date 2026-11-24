//! What a verification answers.
use crate::outbound::evidence::types::RuleTiming;
use benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::collections::BTreeMap;

/// The cases a verification checked and how long each rule's cases took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedCases {
    /// How many cases were checked.
    pub count: CaseCount,
    /// How long each rule's cases took.
    pub timings: BTreeMap<ryo::linter::RuleId, RuleTiming>,
}

/// The execution each contestant runs on: the composition chooses one for the reference and one for the candidate.
#[derive(Clone, Copy)]
pub struct Executions<'a> {
    /// What the reference's invocations run on.
    pub reference: &'a dyn crate::outbound::execution::interface::ExecutionPort,
    /// What the candidate's invocations run on.
    pub candidate: &'a dyn crate::outbound::execution::interface::ExecutionPort,
}
impl std::fmt::Debug for Executions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
