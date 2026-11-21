//! The steps of the basic blocks of a file's functions, in order: the statements, exits and tests that pass control.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;

/// `flow-step`, one item of the `relations` collection: an optional relation, which a language promises or says it does not read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::OptionalCapability;
/// use ryo_relations::domain::relations::collections::relations::flow_step::item::FlowStep;
/// use ryo_relations::domain::relations::service::optional_capability;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let _item = FlowStep;
/// assert_eq!(optional_capability(Relation::FlowStep), Some(OptionalCapability::FlowStep));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FlowStep;
impl super::super::interface::sealed::Sealed for FlowStep {}

/// The declaration of `flow-step`.
#[macro_export]
#[doc(hidden)]
macro_rules! flow_step {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
            item FlowStep,
            row { /// One row of `flow-step`.
                FlowStepRow },
            column { /// The columns of `flow-step`.
                FlowStepColumn },
            expected { /// One expected row of `flow-step`.
                FlowStepExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            form Filled(Whole),
            size PerElement,
            keys [],
            columns {
                /// The block the step is in, by its number in the file.
                block Block: Integer => Number;
                /// The step's place in its block, from zero, steps with no element counted.
                ordinal Ordinal: Integer => Number;
                /// What the step does: statement, return, return-undefined, break, break-labeled, continue, continue-labeled, throw, condition, iteration-of or iteration-in.
                step_kind StepKind: Symbol => Symbol;
                /// The element the step is.
                node Node: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}
crate::flow_step!(crate::relation_row);
