//! The `source_dependency` relation: a path written in one source file that names what another owns; the
//! language's `project` use case writes it, because it takes the modules of every file to say it.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Span};
use crate::domain::terms::types::Symbol;

/// `source_dependency`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct SourceDependency;

impl super::super::interface::sealed::Sealed for SourceDependency {}

/// The declaration of `source_dependency`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! source_dependency {
    ($callback:path) => {
        $callback! {
            item SourceDependency,
            row {
                /// One row of `source_dependency`.
                SourceDependencyRow
            },
            column {
                /// The columns of `source_dependency`.
                SourceDependencyColumn
            },
            expected {
                /// One row of `source_dependency` as a fixture states it.
                SourceDependencyExpectedRow
            },
            class Manifest,
            columns {
                /// The file the path is written in.
                source Source: FileId => File;
                /// The file that owns what the path names.
                target Target: FileId => File;
                /// Where the path is written.
                span Span: Span => Span;
                /// The path as the file writes it, its whitespace left out: the text at `span`. Free text: the spelling is the author's.
                written Written: Symbol => Symbol;
            }
        }
    };
}

crate::source_dependency!(crate::relation_row);
