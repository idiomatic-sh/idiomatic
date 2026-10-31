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

/// The engine-owned reader; every provider delegates to the same indexed source coordinates.
#[derive(Debug)]
pub struct SourceLocationReader<M>(pub std::marker::PhantomData<std::sync::atomic::AtomicPtr<M>>);

impl<M> crate::domain::answers::types::declared::Syntax for SourceLocationReader<M> {}

impl<M: 'static> crate::domain::answers::types::ReadItem for SourceLocationReader<M> {
    type Relation = SourceLocation;
    type Model = M;
    const READER: crate::domain::answers::types::Reader<SourceLocation, M> =
        crate::domain::answers::types::Reader {
            reads: crate::domain::answers::types::Reads::Every,
            visit: &Self(std::marker::PhantomData),
            expansions: crate::domain::answers::types::Expansions::Ignored,
            relation: std::marker::PhantomData,
        };
}

impl<M> crate::domain::answers::types::Visiting<SourceLocation, M> for SourceLocationReader<M> {
    fn visit(
        &self,
        tree: &crate::domain::elements::types::Tree<'_>,
        _model: &M,
        _symbols: &mut crate::domain::symbols::types::Symbols,
        position: crate::domain::relations::types::Position,
        out: &mut dyn crate::domain::relations::types::Rows<SourceLocationRow>,
    ) {
        use crate::domain::elements::service::{end_at, start_at};
        use crate::domain::location::{
            service::{column, line},
            types::SourceLineOffset,
        };
        let span = Span {
            start: start_at(tree.elements, position),
            end: end_at(tree.elements, position),
        };
        if span.start > span.end
            || usize::try_from(span.end.0).unwrap_or(usize::MAX) > tree.source.0.len()
        {
            return;
        }
        let index = &crate::domain::elements::service::engine(tree.elements).line_starts;
        out.write(SourceLocationRow {
            id: crate::domain::relations::service::node_id(tree.file, position),
            file: tree.file,
            span,
            start_line: Integer(line(index, SourceLineOffset(span.start.0)).0),
            end_line: Integer(line(index, SourceLineOffset(span.end.0)).0),
            start_byte: Integer(i64::from(span.start.0)),
            end_byte: Integer(i64::from(span.end.0)),
            start_column: Integer(column(index, SourceLineOffset(span.start.0)).0),
            end_column: Integer(column(index, SourceLineOffset(span.end.0)).0),
        });
    }
}
