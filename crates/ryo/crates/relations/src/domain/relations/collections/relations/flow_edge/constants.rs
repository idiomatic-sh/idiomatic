//! The names of the `flow-edge` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowEdgeColumn;
use crate::domain::relations::constants::{SOURCE, TARGET};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::labels::types::NameRef;
/// use ryo_relations::domain::relations::collections::relations::flow_edge::constants::name;
/// use ryo_relations::domain::relations::service::relation_named;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(relation_named(NameRef(name())), Some(Relation::FlowEdge));
/// ```
#[must_use]
pub fn name() -> &'static str {
    "flow-edge"
}

/// What the relation holds, in a sentence.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_edge::constants::summary;
///
/// assert!(summary().ends_with('.'));
/// ```
#[must_use]
pub fn summary() -> &'static str {
    "The edges between the basic blocks of the control flow graphs of a file's functions."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_edge::constants::column_name;
/// use ryo_relations::domain::relations::collections::relations::flow_edge::item::FlowEdgeColumn;
///
/// assert_eq!(column_name(FlowEdgeColumn::EdgeKind), "edge-kind");
/// ```
#[must_use]
pub fn column_name(column: FlowEdgeColumn) -> &'static str {
    match column {
        FlowEdgeColumn::Source => SOURCE,
        FlowEdgeColumn::Target => TARGET,
        FlowEdgeColumn::EdgeKind => "edge-kind",
        FlowEdgeColumn::File => "file",
    }
}
