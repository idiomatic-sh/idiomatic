//! The `scope` relation: every scope of a file with the scope that directly encloses it. The module of a file is a scope too, and the
//! one that encloses itself: of the scopes of a file, exactly one has its own node for its parent.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};

/// `scope`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Scope;

impl super::super::interface::sealed::Sealed for Scope {}

/// The declaration of `scope`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! scope {
    ($callback:path) => {
        $callback! {
            item Scope,
            row {
                /// One row of `scope`.
                ScopeRow
            },
            column {
                /// The columns of `scope`.
                ScopeColumn
            },
            expected {
                /// One row of `scope` as a fixture states it.
                ScopeExpectedRow
            },
            class Code,
            columns {
                /// The enclosing scope: its file and where it starts; the module of the file, which no scope encloses, is its own.
                parent Parent: NodeId => Node;
                /// The enclosed scope: its file and where it starts.
                child Child: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
                /// Where the enclosed scope is.
                span Span: Span => Span;
            }
        }
    };
}

crate::scope!(crate::relation_row);
