//! The `function` relation: one row for each function declaration of a file.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `function`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Function;

impl super::super::interface::sealed::Sealed for Function {}

/// The declaration of `function`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! function {
    ($callback:path) => {
        $callback! {
            item Function,
            row {
                /// One row of `function`.
                FunctionRow
            },
            column {
                /// The columns of `function`.
                FunctionColumn
            },
            expected {
                /// One row of `function` as a fixture states it.
                FunctionExpectedRow
            },
            class Code,
            columns {
                /// The function: its file and where it starts.
                id Id: NodeId => Node;
                /// Its name.
                name Name: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where it is.
                span Span: Span => Span;
            }
        }
    };
}

crate::function!(crate::relation_row);
