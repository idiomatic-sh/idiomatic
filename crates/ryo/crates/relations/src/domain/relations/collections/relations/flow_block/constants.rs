//! The names of the `flow-block` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowBlockColumn;
use crate::domain::relations::constants::BLOCK;

/// The relation's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::labels::types::NameRef;
/// use ryo_relations::domain::relations::collections::relations::flow_block::constants::name;
/// use ryo_relations::domain::relations::service::relation_named;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(relation_named(NameRef(name())), Some(Relation::FlowBlock));
/// ```
#[must_use]
pub fn name() -> &'static str {
    "flow-block"
}

/// What the relation holds, in a sentence.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_block::constants::summary;
///
/// assert!(summary().ends_with('.'));
/// ```
#[must_use]
pub fn summary() -> &'static str {
    "The basic blocks of the control flow graphs of a file's functions."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_block::constants::column_name;
/// use ryo_relations::domain::relations::collections::relations::flow_block::item::FlowBlockColumn;
///
/// assert_eq!(column_name(FlowBlockColumn::ImplicitReturn), "implicit-return");
/// ```
#[must_use]
pub fn column_name(column: FlowBlockColumn) -> &'static str {
    match column {
        FlowBlockColumn::Block => BLOCK,
        FlowBlockColumn::Reachable => "reachable",
        FlowBlockColumn::Live => "live",
        FlowBlockColumn::ImplicitReturn => "implicit-return",
        FlowBlockColumn::File => "file",
    }
}
