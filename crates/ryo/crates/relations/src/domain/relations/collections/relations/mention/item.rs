//! The `mention` relation: an identifier written inside a macro call's token tree. Never mixed into `reference`: a rule that needs exact uses reads `reference`, a rule that needs to know a name is not used anywhere reads both.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `mention`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Mention;

impl super::super::interface::sealed::Sealed for Mention {}

/// The declaration of `mention`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! mention {
    ($callback:path) => {
        $callback! {
            item Mention,
            row {
                /// One row of `mention`.
                MentionRow
            },
            column {
                /// The columns of `mention`.
                MentionColumn
            },
            expected {
                /// One row of `mention` as a fixture states it.
                MentionExpectedRow
            },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The use: its file and where it starts.
                occurrence Occurrence: NodeId => Node;
                /// What encloses it: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The name used.
                target Target: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the use is.
                span Span: Span => Span;
            }
        }
    };
}

crate::mention!(crate::relation_row);
