//! Lexical indentation regions, independent of syntax-body or name-binding scopes.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Integer, NodeId, Span};
use crate::domain::terms::types::Symbol;
/// A lexical indentation region.
#[derive(Clone, Copy, Debug)]
pub struct IndentRegion;
impl super::super::interface::sealed::Sealed for IndentRegion {}
/// Declare the public indentation-region columns once.
#[macro_export]
#[doc(hidden)]
macro_rules! indent_region {
    ($callback:path) => {
        $callback! {
        item IndentRegion,
        row { /// One lexical indentation interval.
        IndentRegionRow },
        column { /// The columns of an indentation region.
        IndentRegionColumn },
        expected { /// An independently stated indentation region.
        IndentRegionExpectedRow },
        class Lexical,
        columns {
        /// The module node for the root, otherwise the actual INDENT token.
        id Id: NodeId => Node;
        /// The enclosing region's id; the root is its own parent.
        parent Parent: NodeId => Node;
        /// Raw indentation prefix, preserving tabs and form feeds; empty for the root.
        indentation Indentation: Symbol => Symbol;
        /// Zero-based UTF-8 byte offset at INDENT.start, or zero for the root.
        start_byte StartByte: Integer => Number;
        /// Exclusive end at matching DEDENT.end, or EOF for the root.
        end_byte EndByte: Integer => Number;
        /// The containing file.
        file File: FileId => File;
        /// The same half-open source interval.
        span Span: Span => Span;
        }
        }
    };
}
crate::indent_region!(crate::relation_row);
