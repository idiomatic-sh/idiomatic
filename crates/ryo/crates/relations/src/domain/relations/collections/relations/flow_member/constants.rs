//! The names of the `flow-member` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowMemberColumn;
use crate::domain::relations::constants::{BLOCK, NODE};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "flow-member"
}

/// What the relation holds, in a sentence.
#[must_use]
pub(crate) fn summary() -> &'static str {
    "The basic block each element of a file's functions is evaluated in."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub(crate) fn column_name(column: FlowMemberColumn) -> &'static str {
    match column {
        FlowMemberColumn::Node => NODE,
        FlowMemberColumn::Block => BLOCK,
        FlowMemberColumn::File => "file",
    }
}
