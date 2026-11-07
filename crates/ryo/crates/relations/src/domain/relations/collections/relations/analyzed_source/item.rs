//! The `analyzed_source` relation: one row for each source file a language examined in full when it linked the
//! files to one another; the language's `project` use case writes it.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::FileId;

/// `analyzed_source`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct AnalyzedSource;

impl super::super::interface::sealed::Sealed for AnalyzedSource {}

/// The declaration of `analyzed_source`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! analyzed_source {
    ($callback:path) => {
        $callback! {
            item AnalyzedSource,
            row {
                /// One row of `analyzed_source`.
                AnalyzedSourceRow
            },
            column {
                /// The columns of `analyzed_source`.
                AnalyzedSourceColumn
            },
            expected {
                /// One row of `analyzed_source` as a fixture states it.
                AnalyzedSourceExpectedRow
            },
            class Manifest,
            columns {
                /// The file.
                path Path: FileId => File;
            }
        }
    };
}

crate::analyzed_source!(crate::relation_row);
