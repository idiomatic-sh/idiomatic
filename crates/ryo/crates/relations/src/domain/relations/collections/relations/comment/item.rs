//! The `comment` relation: content without delimiters and the full source span.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `comment`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Comment;

impl super::super::interface::sealed::Sealed for Comment {}

/// The declaration of `comment`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! comment {
    ($callback:path) => {
        $callback! {
            item Comment,
            row {
                /// One row of `comment`.
                CommentRow
            },
            column {
                /// The columns of `comment`.
                CommentColumn
            },
            expected {
                /// One row of `comment` as a fixture states it.
                CommentExpectedRow
            },
            class Lexical,
            columns {
                /// The comment: its file and where it starts.
                id Id: NodeId => Node;
                /// Its content, without the delimiters. Free text: the words of a comment are the author's.
                text Text: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where it is, delimiters included.
                span Span: Span => Span;
                /// Whether it is documentation by its language's own convention: a comment the language's tools read as the documentation of
                /// what follows it or of what encloses it, as each language's folder says. Lowered for every other comment, and for every comment
                /// of a language with no such convention.
                doc Doc: Flag => Flag;
            }
        }
    };
}

crate::comment!(crate::relation_row);
