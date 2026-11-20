//! The edges between the basic blocks of the control flow graphs of a file's functions.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer};
use crate::domain::terms::types::Symbol;

/// `flow-edge`, one item of the `relations` collection: an optional relation, which a language promises or says it does not read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::OptionalCapability;
/// use ryo_relations::domain::relations::collections::relations::flow_edge::item::FlowEdge;
/// use ryo_relations::domain::relations::service::optional_capability;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let _item = FlowEdge;
/// assert_eq!(optional_capability(Relation::FlowEdge), Some(OptionalCapability::FlowEdge));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FlowEdge;
impl super::super::interface::sealed::Sealed for FlowEdge {}

/// The declaration of `flow-edge`.
#[macro_export]
#[doc(hidden)]
macro_rules! flow_edge {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
            item FlowEdge,
            row { /// One row of `flow-edge`.
                FlowEdgeRow },
            column { /// The columns of `flow-edge`.
                FlowEdgeColumn },
            expected { /// One expected row of `flow-edge`.
                FlowEdgeExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            form Filled(Whole),
            columns {
                /// The block the edge leaves, by its number in the file.
                source Source: Integer => Number;
                /// The block the edge enters, by its number in the file.
                target Target: Integer => Number;
                /// How control passes: normal, jump, backedge, join, finalize, new-function, unreachable, error-explicit or error-implicit.
                edge_kind EdgeKind: Symbol => Symbol;
                /// The edge's place among the edges leaving its source, in the order the language's flow analysis follows them, from 0.
                ordinal Ordinal: Integer => Number;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}
crate::flow_edge!(crate::relation_row);
