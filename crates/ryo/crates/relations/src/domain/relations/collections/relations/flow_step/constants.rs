//! The names of the `flow-step` relation: the text its name and its columns carry when they leave the engine.
use super::item::FlowStepColumn;
use crate::domain::relations::constants::{BLOCK, NODE, ORDINAL};

/// The relation's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::labels::types::NameRef;
/// use ryo_relations::domain::relations::collections::relations::flow_step::constants::name;
/// use ryo_relations::domain::relations::service::relation_named;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_eq!(relation_named(NameRef(name())), Some(Relation::FlowStep));
/// ```
#[must_use]
pub fn name() -> &'static str {
    "flow-step"
}

/// What the relation holds, in a sentence.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_step::constants::summary;
///
/// assert!(summary().ends_with('.'));
/// ```
#[must_use]
pub fn summary() -> &'static str {
    "The steps of the basic blocks of a file's functions, in order: the statements, exits and tests that pass control."
}

/// A column's name in RyoQL and in listings: text that leaves the engine.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::flow_step::constants::column_name;
/// use ryo_relations::domain::relations::collections::relations::flow_step::item::FlowStepColumn;
///
/// assert_eq!(column_name(FlowStepColumn::StepKind), "step-kind");
/// ```
#[must_use]
pub fn column_name(column: FlowStepColumn) -> &'static str {
    match column {
        FlowStepColumn::Block => BLOCK,
        FlowStepColumn::Ordinal => ORDINAL,
        FlowStepColumn::StepKind => "step-kind",
        FlowStepColumn::Node => NODE,
        FlowStepColumn::File => "file",
    }
}
