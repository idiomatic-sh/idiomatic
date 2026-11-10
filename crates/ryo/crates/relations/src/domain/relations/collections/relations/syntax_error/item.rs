//! The `syntax_error` relation: a diagnostic of the parser, kept beside the recovered tree.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `syntax_error`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct SyntaxError;

impl super::super::interface::sealed::Sealed for SyntaxError {}

/// The declaration of `syntax_error`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! syntax_error {
    ($callback:path) => {
        $callback! {
            item SyntaxError,
            row {
                /// One row of `syntax_error`.
                SyntaxErrorRow
            },
            column {
                /// The columns of `syntax_error`.
                SyntaxErrorColumn
            },
            expected {
                /// One row of `syntax_error` as a fixture states it.
                SyntaxErrorExpectedRow
            },
            class Structure,
            columns {
                /// The error: its file and where it is.
                id Id: NodeId => Node;
                /// The parser's message. Free text: the parser's sentence.
                message Message: Symbol => Symbol;
                /// The stage that reported this diagnostic.
                phase Phase: Symbol => Symbol;
                /// Whether semantic facts remain complete despite this diagnostic.
                impact Impact: Symbol => Symbol;
                /// The stable code owned by the language provider.
                code Code: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the error is.
                span Span: Span => Span;
            }
        }
    };
}

crate::syntax_error!(crate::relation_row);
