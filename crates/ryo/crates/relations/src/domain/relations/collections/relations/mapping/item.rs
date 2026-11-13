//! The `mapping` relation: one row for each mapping of a data document.

use super::super::interface::RelationsItem;
use crate::domain::mapping_kind::types::MappingKind;
use crate::domain::relations::types::{FileId, Integer, NodeId};
use crate::domain::terms::service::term_symbol;

/// `mapping`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct Mapping;

impl super::super::interface::sealed::Sealed for Mapping {}

/// The declaration of `mapping`: its columns, named once, handed to the macro that makes of them what a relation is.
#[macro_export]
#[doc(hidden)]
macro_rules! mapping {
    ($callback:path) => {
        $callback! {
            item Mapping,
            row {
                /// One row of `mapping`.
                MappingRow
            },
            column {
                /// The columns of `mapping`.
                MappingColumn
            },
            expected {
                /// One row of `mapping` as a fixture states it.
                MappingExpectedRow
            },
            class Keyed,
            scope File,
            notes Unneeded,
            columns {
                /// The mapping: its file and where it starts.
                id Id: NodeId => Node;
                /// How it is written.
                kind Kind: MappingKind => Symbol(term_symbol);
                /// Its place in a sequence of mappings, otherwise zero.
                ordinal Ordinal: Integer => Number;
                /// The entry or sequence element that holds it: its file and where it starts.
                owner Owner: NodeId => Node;
                /// The file it is in.
                file File: FileId => File;
            }
        }
    };
}

crate::mapping!(crate::relation_row);
