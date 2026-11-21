//! The services of the `nodes` part: a node row's handle and its cells. A handle is the element's id as every relation
//! names it: the file in the high half, the element's place in the file's table in the low. A cell of a column is read from
//! the table by the place; a kind and a field are the word of their name, a text the word of the bytes the element spans.

use crate::domain::stages::types::NodeNames;
use ryo_language::domain::facts::service::intern;
use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_language::domain::node_table::service::{
    field_of, kind_of, ordinal_of, parent_of, span_of,
};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::service::{node_id, node_position};
use ryo_relations::domain::relations::types::{
    Count, FileId, NodeColumn, NodeId, Offset, Position,
};
use ryo_relations::domain::terms::types::NameText;

/// The cell of a kind or of a field whose code the vocabulary names nothing at: no word of any text.
const NO_WORD: CellWord = CellWord(u64::MAX);

/// The bits every handle of a node of `file` carries besides the element's place: the file, in the high half.
#[inline]
pub(crate) fn node_tag(file: FileId) -> CellWord {
    CellWord(node_id(file, Position(0)).0)
}

/// The element a node's handle names.
#[inline]
pub(crate) fn position_of(handle: CellWord) -> Position {
    node_position(NodeId(handle.0))
}

/// The word of a name by its code; no word for a code the vocabulary names nothing at.
#[inline]
fn named(names: &[CellWord], code: Count) -> CellWord {
    match names.get(code.0) {
        Some(word) => *word,
        None => NO_WORD,
    }
}

/// The cell of a place in the file: an ordinal, an offset.
#[inline]
fn numbered(value: Offset) -> CellWord {
    CellWord(u64::from(value.0))
}

/// The cell of one column of one element.
#[inline]
pub(crate) fn node_cell(
    nodes: &NodeTable,
    names: NodeNames<'_>,
    words: &mut FileWords,
    column: NodeColumn,
    handle: CellWord,
) -> CellWord {
    let at = position_of(handle);
    match column {
        NodeColumn::Id => handle,
        NodeColumn::Kind => named(names.kinds, Count(usize::from(kind_of(nodes, at).0))),
        NodeColumn::Parent => CellWord(node_id(names.file, parent_of(nodes, at)).0),
        NodeColumn::Field => named(names.fields, Count(usize::from(field_of(nodes, at).0))),
        NodeColumn::Ordinal => CellWord(u64::from(ordinal_of(nodes, at).0)),
        NodeColumn::Start => numbered(span_of(nodes, at).start),
        NodeColumn::End => numbered(span_of(nodes, at).end),
        NodeColumn::Span => {
            let span = span_of(nodes, at);
            CellWord((u64::from(span.start.0) << 32) | u64::from(span.end.0))
        }
        NodeColumn::File => CellWord(u64::from(names.file.0)),
        NodeColumn::Text => {
            let span = span_of(nodes, at);
            let (start, end) = (
                usize::try_from(span.start.0).unwrap_or(usize::MAX),
                usize::try_from(span.end.0).unwrap_or(usize::MAX),
            );
            let bytes = names.text.0.get(start..end).unwrap_or_default();
            intern(words, NameText(&String::from_utf8_lossy(bytes)))
        }
    }
}

/// `Read` over the node table: for each handle of the batch, the cell of the element's column. One loop for each column.
pub(crate) fn node_column(
    nodes: &NodeTable,
    names: NodeNames<'_>,
    words: &mut FileWords,
    column: NodeColumn,
    handles: &[CellWord],
    out: &mut Vec<CellWord>,
) {
    out.clear();
    match column {
        NodeColumn::Id => out.extend_from_slice(handles),
        NodeColumn::Kind => out.extend(handles.iter().map(|each| {
            named(
                names.kinds,
                Count(usize::from(kind_of(nodes, position_of(*each)).0)),
            )
        })),
        NodeColumn::Field => out.extend(handles.iter().map(|each| {
            named(
                names.fields,
                Count(usize::from(field_of(nodes, position_of(*each)).0)),
            )
        })),
        NodeColumn::Parent => out.extend(
            handles
                .iter()
                .map(|each| CellWord(node_id(names.file, parent_of(nodes, position_of(*each))).0)),
        ),
        NodeColumn::Ordinal => out.extend(
            handles
                .iter()
                .map(|each| CellWord(u64::from(ordinal_of(nodes, position_of(*each)).0))),
        ),
        NodeColumn::Start => out.extend(
            handles
                .iter()
                .map(|each| numbered(span_of(nodes, position_of(*each)).start)),
        ),
        NodeColumn::End => out.extend(
            handles
                .iter()
                .map(|each| numbered(span_of(nodes, position_of(*each)).end)),
        ),
        NodeColumn::Span | NodeColumn::File | NodeColumn::Text => {
            for each in handles {
                out.push(node_cell(nodes, names, words, column, *each));
            }
        }
    }
}

/// One column of the node table as cells, element after element up to before `end`: what an index over the node table
/// is built from.
pub(crate) fn node_cells(
    nodes: &NodeTable,
    names: NodeNames<'_>,
    words: &mut FileWords,
    column: NodeColumn,
    end: Position,
    out: &mut Vec<CellWord>,
) {
    let tag = node_tag(names.file).0;
    out.clear();
    out.extend(
        (0..end.0).map(|at| node_cell(nodes, names, words, column, CellWord(tag | u64::from(at)))),
    );
}
