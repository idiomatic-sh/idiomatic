//! The names of the `flow-block` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowBlockColumn;
use crate::domain::relations::constants::BLOCK;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "flow-block"
}

/// What the relation holds, in a sentence.
#[must_use]
pub(crate) fn summary() -> &'static str {
    "The basic blocks of the control flow graphs of a file's functions."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub(crate) fn column_name(column: FlowBlockColumn) -> &'static str {
    match column {
        FlowBlockColumn::Block => BLOCK,
        FlowBlockColumn::Reachable => "reachable",
        FlowBlockColumn::Live => "live",
        FlowBlockColumn::ImplicitReturn => "implicit-return",
        FlowBlockColumn::File => "file",
    }
}
