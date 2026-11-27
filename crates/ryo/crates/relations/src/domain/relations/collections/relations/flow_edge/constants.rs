//! The names of the `flow-edge` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowEdgeColumn;
use crate::domain::relations::constants::{ORDINAL, SOURCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub fn name() -> &'static str {
    "flow-edge"
}

/// What the relation holds, in a sentence.
#[must_use]
pub(crate) fn summary() -> &'static str {
    "The edges between the basic blocks of the control flow graphs of a file's functions."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
#[must_use]
pub(crate) fn column_name(column: FlowEdgeColumn) -> &'static str {
    match column {
        FlowEdgeColumn::Source => SOURCE,
        FlowEdgeColumn::Target => TARGET,
        FlowEdgeColumn::EdgeKind => "edge-kind",
        FlowEdgeColumn::Ordinal => ORDINAL,
        FlowEdgeColumn::File => "file",
    }
}
