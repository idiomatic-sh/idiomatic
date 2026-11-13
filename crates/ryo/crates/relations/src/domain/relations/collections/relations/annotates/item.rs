//! The `annotates` relation: the type a parameter, a function result or a single-name `let` is annotated with, as written.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `annotates`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Annotates;

impl super::super::interface::sealed::Sealed for Annotates {}

/// The declaration of `annotates`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! annotates {
    ($callback:path) => {
        $callback! {
            item Annotates,
            row {
                /// One row of `annotates`.
                AnnotatesRow
            },
            column {
                /// The columns of `annotates`.
                AnnotatesColumn
            },
            expected {
                /// One row of `annotates` as a fixture states it.
                AnnotatesExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The annotation: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// What it annotates: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The type, as written.
                annotation Annotation: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the annotation is.
                span Span: Span => Span;
            }
        }
    };
}

crate::annotates!(crate::relation_row);
