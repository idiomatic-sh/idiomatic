//! The `decorates` relation: an attribute of a declaration that carries one, by the path it names and the arguments it passes, in its place.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `decorates`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Decorates;

impl super::super::interface::sealed::Sealed for Decorates {}

/// The declaration of `decorates`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! decorates {
    ($callback:path) => {
        $callback! {
            item Decorates,
            row {
                /// One row of `decorates`.
                DecoratesRow
            },
            column {
                /// The columns of `decorates`.
                DecoratesColumn
            },
            expected {
                /// One row of `decorates` as a fixture states it.
                DecoratesExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The attribute: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// The declaration it decorates: its file and where it starts.
                declaration Declaration: NodeId => Node;
                /// Its place among the declaration's attributes, from zero.
                ordinal Ordinal: Integer => Number;
                /// The path it names.
                target Target: Symbol => Symbol;
                /// The arguments it passes, as written between its delimiters (`Clone, Debug` of `#[derive(Clone, Debug)]`), the value of
                /// `name = value`, or empty when it passes none. Free text: the words are the author's.
                arguments Arguments: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the attribute is.
                span Span: Span => Span;
            }
        }
    };
}

crate::decorates!(crate::relation_row);
