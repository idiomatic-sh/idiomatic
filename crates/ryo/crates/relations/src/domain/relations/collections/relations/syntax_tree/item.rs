//! The `syntax_tree` relation: a node or a token of the syntax tree, whitespace and comments included.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `syntax_tree`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct SyntaxTree;

impl super::super::interface::sealed::Sealed for SyntaxTree {}

/// The declaration of `syntax_tree`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! syntax_tree {
    ($callback:path) => {
        $callback! {
            item SyntaxTree,
            row {
                /// One row of `syntax_tree`.
                SyntaxTreeRow
            },
            column {
                /// The columns of `syntax_tree`.
                SyntaxTreeColumn
            },
            expected {
                /// One row of `syntax_tree` as a fixture states it.
                SyntaxTreeExpectedRow
            },
            class Structure,
            scope File,
            notes Unneeded,
            columns {
                /// The element: its file and where it starts.
                id Id: NodeId => Node;
                /// What it is, in the language's own words.
                kind Kind: Symbol => Symbol;
                /// A token's exact text; nothing for a node. Source text: the token as it is in the file.
                text Text: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where it is.
                span Span: Span => Span;
            }
        }
    };
}

crate::syntax_tree!(crate::relation_row);
