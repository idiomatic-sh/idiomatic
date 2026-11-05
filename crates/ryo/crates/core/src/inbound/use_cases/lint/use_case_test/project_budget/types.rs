//! The types of the `project_budget` use case: plain data, public fields and derives, with no constructor and no inherent impl.
//! Plain input and observed work for the real lint project-budget controls.
use crate::inbound::use_cases::lint::{errors::LintError, types::LintReport};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BudgetRoute {
    Ordinary,
    Named,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MemoryPressure {
    Budget,
    Unreadable,
    Allowed,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct BudgetScenario {
    pub route: BudgetRoute,
    pub pressure: MemoryPressure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReductionCount(pub usize);
pub(super) struct BudgetObservation {
    pub report: Result<LintReport, LintError>,
    pub reductions: ReductionCount,
}
