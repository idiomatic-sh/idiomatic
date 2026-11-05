//! The types of the `answering` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::render::types::Rendered;
use ryo_execution::inbound::use_cases::execution::types::RowsFed;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_values::domain::runs::types::RuleId;

/// A rule whose cross-file steps are made: its place among the rules and its id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Todo {
    /// The rule, by its place among the rules.
    pub rule: RuleIndex,
    /// The rule's id.
    pub id: RuleId,
}

/// What the cross-file steps of a rule found, and the rows each step was fed.
#[derive(Clone, Debug, Default)]
pub(crate) struct Crossed {
    /// Actual consuming-rule coverage failures, separate from diagnostic rows.
    pub unknowns: Vec<ryo_execution::inbound::use_cases::metadata::interface::Unknown>,
    /// The findings, as typed cells.
    pub rendered: Option<Rendered>,
    /// Owned named-root publication bytes, never user diagnostics.
    pub observations: Option<ryo_values::domain::records::types::CrossPublicationTransfer>,
    /// The rows each step of the plan was fed.
    pub fed: Vec<RowsFed>,
}

/// Whether a plan has cross-file steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CrossSteps {
    /// It has.
    Has,
    /// Every step runs on one file.
    Lacks,
}

/// What a run answers when its rows are written: what the cross-file steps found, and the rows each step was fed.
pub(crate) struct Answered {
    pub unknowns: Vec<ryo_execution::inbound::use_cases::metadata::interface::Unknown>,
    /// The rows written after the files.
    pub later: Rendered,
    /// The rows each step of each rule was fed, by rule.
    pub fed: Vec<Vec<RowsFed>>,
}
