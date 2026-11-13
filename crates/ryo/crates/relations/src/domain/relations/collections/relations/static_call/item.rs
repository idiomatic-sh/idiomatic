//! The `static_call` relation: a call of a path written out, by the path it names.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `static_call`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct StaticCall;

impl super::super::interface::sealed::Sealed for StaticCall {}

/// The declaration of `static_call`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! static_call {
    ($callback:path) => {
        $callback! {
            item StaticCall,
            row {
                /// One row of `static_call`.
                StaticCallRow
            },
            column {
                /// The columns of `static_call`.
                StaticCallColumn
            },
            expected {
                /// One row of `static_call` as a fixture states it.
                StaticCallExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The call: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// What encloses it: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The callee, normalized.
                target Target: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the callee is.
                span Span: Span => Span;
            }
        }
    };
}

crate::static_call!(crate::relation_row);
