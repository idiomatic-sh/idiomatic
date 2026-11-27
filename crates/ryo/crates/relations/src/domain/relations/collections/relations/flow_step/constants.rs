//! The names of the `flow-step` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowStepColumn;
use crate::domain::relations::constants::{BLOCK, NODE, ORDINAL};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "flow-step"
}

/// What the relation holds, in a sentence.
#[must_use]
pub(crate) fn summary() -> &'static str {
    "The steps of the basic blocks of a file's functions, in order: the statements, exits and tests that pass control."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub(crate) fn column_name(column: FlowStepColumn) -> &'static str {
    match column {
        FlowStepColumn::Block => BLOCK,
        FlowStepColumn::Ordinal => ORDINAL,
        FlowStepColumn::StepKind => "step-kind",
        FlowStepColumn::Node => NODE,
        FlowStepColumn::File => "file",
    }
}
