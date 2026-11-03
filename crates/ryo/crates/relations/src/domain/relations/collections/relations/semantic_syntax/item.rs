//! The `semantic_syntax` relation: a node or a token of the syntax tree, whitespace and comments included.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `semantic_syntax`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct SemanticSyntax;

impl super::super::interface::sealed::Sealed for SemanticSyntax {}

/// The declaration of `semantic_syntax`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! semantic_syntax {
    ($callback:path) => {
        $callback! {
            item SemanticSyntax,
            row {
                /// One row of `semantic_syntax`.
                SemanticSyntaxRow
            },
            column {
                /// The columns of `semantic_syntax`.
                SemanticSyntaxColumn
            },
            expected {
                /// One row of `semantic_syntax` as a fixture states it.
                SemanticSyntaxExpectedRow
            },
            class Structure,
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

crate::semantic_syntax!(crate::relation_row);
