//! The `assigns` relation: the string literal a single-name `let` binds, linked to the binding and the literal.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};

/// `assigns`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Assigns;

impl super::super::interface::sealed::Sealed for Assigns {}

/// The declaration of `assigns`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! assigns {
    ($callback:path) => {
        $callback! {
            item Assigns,
            row {
                /// One row of `assigns`.
                AssignsRow
            },
            column {
                /// The columns of `assigns`.
                AssignsColumn
            },
            expected {
                /// One row of `assigns` as a fixture states it.
                AssignsExpectedRow
            },
            class Code,
            columns {
                /// The assignment: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// The binding: its file and where it starts.
                binding Binding: NodeId => Node;
                /// The literal: its file and where it starts.
                value Value: NodeId => Node;
                /// The function that holds it, or the file's start.
                owner Owner: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
                /// Where the value is.
                span Span: Span => Span;
            }
        }
    };
}

crate::assigns!(crate::relation_row);
