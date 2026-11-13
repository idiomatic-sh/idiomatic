//! The `directive` relation: an instruction to the build tool that is written in a source file, by its kind and its text as the tool
//! reads it.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Span};
use crate::domain::terms::types::Symbol;

/// `directive`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Directive;

impl super::super::interface::sealed::Sealed for Directive {}

/// The declaration of `directive`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! directive {
    ($callback:path) => {
        $callback! {
            item Directive,
            row {
                /// One row of `directive`.
                DirectiveRow
            },
            column {
                /// The columns of `directive`.
                DirectiveColumn
            },
            expected {
                /// One row of `directive` as a fixture states it.
                DirectiveExpectedRow
            },
            class Literal,
            scope File,
            notes Unneeded,
            columns {
                /// The file it is in.
                file File: FileId => File;
                /// Where it is, as the language writes it.
                span Span: Span => Span;
                /// The directive's name as it is written, the word the language gives it. Free text: the language owns the words.
                kind Kind: Symbol => Symbol;
                /// Its argument text, as the build tool reads it. Free text: the words are the author's.
                text Text: Symbol => Symbol;
            }
        }
    };
}

crate::directive!(crate::relation_row);
