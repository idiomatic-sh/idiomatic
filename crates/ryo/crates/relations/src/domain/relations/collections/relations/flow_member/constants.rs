//! The names of the `flow-member` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowMemberColumn;
use crate::domain::relations::constants::{BLOCK, NODE};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::labels::types::NameRef;
/// use ryo_relations::domain::relations::collections::relations::flow_member::constants::name;
/// use ryo_relations::domain::relations::service::relation_named;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(relation_named(NameRef(name())), Some(Relation::FlowMember));
/// ```
#[must_use]
pub fn name() -> &'static str {
    "flow-member"
}

/// What the relation holds, in a sentence.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_member::constants::summary;
///
/// assert!(summary().ends_with('.'));
/// ```
#[must_use]
pub fn summary() -> &'static str {
    "The basic block each element of a file's functions is evaluated in."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_member::constants::column_name;
/// use ryo_relations::domain::relations::collections::relations::flow_member::item::FlowMemberColumn;
///
/// assert_eq!(column_name(FlowMemberColumn::Block), "block");
/// ```
#[must_use]
pub fn column_name(column: FlowMemberColumn) -> &'static str {
    match column {
        FlowMemberColumn::Node => NODE,
        FlowMemberColumn::Block => BLOCK,
        FlowMemberColumn::File => "file",
    }
}
