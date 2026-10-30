//! The types of the `answering` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::types::RowsFed;
use crate::domain::render::types::Rendered;
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
    /// The findings, as typed cells.
    pub rendered: Rendered,
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
    /// The rows written after the files.
    pub later: Rendered,
    /// The rows each step of each rule was fed, by rule.
    pub fed: Vec<Vec<RowsFed>>,
}
