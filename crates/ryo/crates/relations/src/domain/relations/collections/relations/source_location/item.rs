//! The `source-location` relation: source coordinates shared by all language readers.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};

/// Source coordinates for a node or token.
#[derive(Clone, Copy, Debug)]
pub struct SourceLocation;

impl super::super::interface::sealed::Sealed for SourceLocation {}

/// Declares the columns of source locations.
#[macro_export]
#[doc(hidden)]
macro_rules! source_location {
    ($callback:path) => {
        $callback! {
            item SourceLocation,
            row { /// One source location.
                SourceLocationRow },
            column { /// The columns of a source location.
                SourceLocationColumn },
            expected { /// An expected source location.
                SourceLocationExpectedRow },
            class Structure,
            columns {
                /// The node or token.
                id Id: NodeId => Node;
                /// Its file.
                file File: FileId => File;
                /// Its half-open byte range.
                span Span: Span => Span;
                /// The one-based line of the start endpoint.
                start_line StartLine: Integer => Number;
                /// The one-based line of the exclusive end endpoint.
                end_line EndLine: Integer => Number;
                /// The zero-based UTF-8 byte offset of the start endpoint.
                start_byte StartByte: Integer => Number;
                /// The zero-based UTF-8 byte offset of the exclusive end endpoint.
                end_byte EndByte: Integer => Number;
                /// The zero-based UTF-8 byte column of the start endpoint.
                start_column StartColumn: Integer => Number;
                /// The zero-based UTF-8 byte column of the exclusive end endpoint.
                end_column EndColumn: Integer => Number;
            }
        }
    };
}

crate::source_location!(crate::relation_row);
