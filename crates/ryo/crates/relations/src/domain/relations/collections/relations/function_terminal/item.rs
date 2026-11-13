//! Terminal behavior of function bodies.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

#[derive(Clone, Copy, Debug)]
/// Basic terminal outcomes of function bodies.
pub struct FunctionTerminal;
impl super::super::interface::sealed::Sealed for FunctionTerminal {}

#[macro_export]
#[doc(hidden)]
macro_rules! function_terminal {
    ($callback:path) => {
        $callback! {
            item FunctionTerminal,
            row {
                /// One function terminal outcome.
                FunctionTerminalRow
            },
            column {
                /// Columns of `function-terminal`.
                FunctionTerminalColumn
            },
            expected {
                /// Expected `function-terminal` row.
                FunctionTerminalExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// Function or async-function syntax node.
                id Id: NodeId => Node;
                /// `implicit`, `raise`, `return-or-raise`, or `conditional-return`.
                outcome Outcome: Symbol => Symbol;
                /// Containing file.
                file File: FileId => File;
                /// Function definition span.
                span Span: Span => Span;
            }
        }
    };
}
crate::function_terminal!(crate::relation_row);
