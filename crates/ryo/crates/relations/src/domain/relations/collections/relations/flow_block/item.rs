//! The basic blocks of the control flow graphs of a file's functions.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, Integer};

/// `flow-block`, one item of the `relations` collection: an optional relation, which a language promises or says it does not read.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::catalog::types::OptionalCapability;
/// use ryo_relations::domain::relations::collections::relations::flow_block::item::FlowBlock;
/// use ryo_relations::domain::relations::service::optional_capability;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let _item = FlowBlock;
/// assert_eq!(optional_capability(Relation::FlowBlock), Some(OptionalCapability::FlowBlock));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FlowBlock;
impl super::super::interface::sealed::Sealed for FlowBlock {}

/// The declaration of `flow-block`.
#[macro_export]
#[doc(hidden)]
macro_rules! flow_block {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
            item FlowBlock,
            row { /// One row of `flow-block`.
                FlowBlockRow },
            column { /// The columns of `flow-block`.
                FlowBlockColumn },
            expected { /// One expected row of `flow-block`.
                FlowBlockExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            form Filled(Whole),
            columns {
                /// The block, by its number in the file.
                block Block: Integer => Number;
                /// Whether the graph's builder marks the block reachable: lowered after a return, throw, break or continue.
                reachable Reachable: Flag => Flag;
                /// Whether control can reach the block once a loop whose condition is the literal `true` exits only through a break.
                live Live: Flag => Flag;
                /// Whether the block ends its function by falling off its end.
                implicit_return ImplicitReturn: Flag => Flag;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}
crate::flow_block!(crate::relation_row);
