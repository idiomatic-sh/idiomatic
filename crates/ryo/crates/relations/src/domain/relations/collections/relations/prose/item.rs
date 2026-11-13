//! The `prose` relation: the text a reader of a document reads, in runs: a paragraph, an item of a list, the title of a heading, a cell of
//! a table, with its inline markup joined and its code and raw markup masked.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `prose`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Prose;

impl super::super::interface::sealed::Sealed for Prose {}

/// The declaration of `prose`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! prose {
    ($callback:path) => {
        $callback! {
            item Prose,
            row {
                /// One row of `prose`.
                ProseRow
            },
            column {
                /// The columns of `prose`.
                ProseColumn
            },
            expected {
                /// One row of `prose` as a fixture states it.
                ProseExpectedRow
            },
            class Document,
            scope File,
            notes Unneeded,
            columns {
                /// The run: its file and where it starts.
                id Id: NodeId => Node;
                /// What encloses it: the section it is in, or the document: its file and where it starts.
                owner Owner: NodeId => Node;
                /// What a reader reads: the words of the run with its markup joined, its code and raw markup masked. Free text: the words are the author's.
                text Text: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the run is.
                span Span: Span => Span;
            }
        }
    };
}

crate::prose!(crate::relation_row);
