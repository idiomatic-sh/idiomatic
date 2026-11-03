//! The `semantic_child` relation: an edge of the syntax tree, ordered under its parent.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;

/// `semantic_child`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct SemanticChild;

impl super::super::interface::sealed::Sealed for SemanticChild {}

/// The declaration of `semantic_child`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! semantic_child {
    ($callback:path) => {
        $callback! {
            item SemanticChild,
            row {
                /// One row of `semantic_child`.
                SemanticChildRow
            },
            column {
                /// The columns of `semantic_child`.
                SemanticChildColumn
            },
            expected {
                /// One row of `semantic_child` as a fixture states it.
                SemanticChildExpectedRow
            },
            class Structure,
            columns {
                /// The parent node: its file and where it starts.
                parent Parent: NodeId => Node;
                /// The child, a node or a token: its file and where it starts.
                child Child: NodeId => Node;
                /// The child's place under the parent, from zero.
                ordinal Ordinal: Integer => Number;
                /// The child's role under the parent, in the language's own vocabulary of roles.
                field Field: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}

crate::semantic_child!(crate::relation_row);
