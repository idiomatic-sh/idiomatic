//! The basic block each element of a file's functions is evaluated in.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId};

/// `flow-member`, one item of the `relations` collection: an optional relation, which a language promises or says it does not read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::OptionalCapability;
/// use ryo_relations::domain::relations::collections::relations::flow_member::item::FlowMember;
/// use ryo_relations::domain::relations::service::optional_capability;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let _item = FlowMember;
/// assert_eq!(optional_capability(Relation::FlowMember), Some(OptionalCapability::FlowMember));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FlowMember;
impl super::super::interface::sealed::Sealed for FlowMember {}

/// The declaration of `flow-member`.
#[macro_export]
#[doc(hidden)]
macro_rules! flow_member {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
            item FlowMember,
            row { /// One row of `flow-member`.
                FlowMemberRow },
            column { /// The columns of `flow-member`.
                FlowMemberColumn },
            expected { /// One expected row of `flow-member`.
                FlowMemberExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            form Filled(Many(Node)),
            columns {
                /// The element.
                node Node: NodeId => Node;
                /// The block it is evaluated in, by its number in the file.
                block Block: Integer => Number;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}
crate::flow_member!(crate::relation_row);
