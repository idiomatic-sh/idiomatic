//! The `call_argument` relation: an argument of a call of a path written out, by its place among the arguments.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `call_argument`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct CallArgument;

impl super::super::interface::sealed::Sealed for CallArgument {}

/// The declaration of `call_argument`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! call_argument {
    ($callback:path) => {
        $callback! {
            item CallArgument,
            row {
                /// One row of `call_argument`.
                CallArgumentRow
            },
            column {
                /// The columns of `call_argument`.
                CallArgumentColumn
            },
            expected {
                /// One row of `call_argument` as a fixture states it.
                CallArgumentExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The call: the `occurrence` of its `static_call` row.
                call Call: NodeId => Node;
                /// The place of the parameter it is bound to among the callee's parameters, from zero, as the call's syntax says it: in a language
                /// whose methods take their receiver as a parameter and whose method calls say so, the receiver written before the dot is
                /// position zero; in the others, its place among the arguments in the parentheses.
                position Position: Integer => Number;
                /// The argument's expression: its file and where it starts.
                expression Expression: NodeId => Node;
                /// The name the argument hands on, when it is a name and nothing else, as written; a name behind a reference, a dereference or a
                /// spread is the name. The empty text when the argument is anything else.
                passes Passes: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the argument is.
                span Span: Span => Span;
            }
        }
    };
}

crate::call_argument!(crate::relation_row);
