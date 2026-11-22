//! The services of the `nodes` part: a node row's handle and its cells. A handle is the element's id as every relation
//! names it: the file in the high half, the element's place in the file's table in the low. A cell of a column is read from
//! the table by the place; a kind and a field are the word of their name, a text the word of the bytes the element spans.

use crate::domain::stages::service::tagged;
use crate::domain::stages::types::NodeNames;
use ryo_language::domain::facts::service::intern;
use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_language::domain::node_table::service::{
    decoded_of, embedded_segments, field_of, kind_of, node_count, ordinal_of, own_end, parent_of, span_of, subtree_end_of,
};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::service::{node_id, node_position, to_bits};
use ryo_relations::domain::relations::types::{
    Count, FileId, NodeColumn, NodeId, Offset, Position, Span, Value,
};
use ryo_relations::domain::terms::types::NameText;

/// The cell that is no word of any text: of a kind or a field whose code the vocabulary names nothing at, and what a
/// function of texts gives of a cell that is no text.
pub(crate) const NO_WORD: CellWord = CellWord(u64::MAX);

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

/// Whether an element is a token: the file's own tree holds its nodes, then its tokens, and each embedded segment the
/// nodes under its root, then its tokens.
fn token(nodes: &NodeTable, at: Position) -> bool {
    let own = own_end(nodes);
    if at < own {
        return u64::from(at.0) >= u64::try_from(node_count(nodes).0).unwrap_or(u64::MAX);
    }
    embedded_segments(nodes)
        .find(|segment| segment.root <= at && at < segment.end)
        .is_some_and(|segment| at >= subtree_end_of(nodes, segment.root))
}

/// The cell of one column of one element.
#[inline]
pub(crate) fn node_cell(
    nodes: &NodeTable,
    names: NodeNames<'_>,
    words: &mut FileWords,
    texts: &mut Vec<CellWord>,
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
            CellWord(
                to_bits(Value::Span(Span {
                    start: span.start,
                    end: span.end,
                }))
                .0,
            )
        }
        NodeColumn::File => CellWord(to_bits(Value::File(names.file)).0),
        NodeColumn::Text => {
            // The word of an element's text is found once a file: `texts` holds it by the element's place (the
            // cell that is no word until then), as many cells as the file has elements.
            let place = usize::try_from(at.0).unwrap_or(usize::MAX);
            if texts.len() <= place {
                let count = usize::try_from(node_count(nodes).0).unwrap_or(usize::MAX);
                texts.resize(count.max(place.saturating_add(1)), NO_WORD);
            }
            if let Some(held) = texts.get(place)
                && *held != NO_WORD
            {
                return *held;
            }
            // A token of an embedded segment is the text the language read there (an escape decoded, the parts of a
            // concatenation joined), not the bytes of the host span it is placed at.
            if at >= own_end(nodes)
                && token(nodes, at)
                && let Some(text) = decoded_of(nodes, at)
            {
                let word = intern(words, text);
                if let Some(held) = texts.get_mut(place) {
                    *held = word;
                }
                return word;
            }
            let span = span_of(nodes, at);
            let (start, end) = if token(nodes, at) {
                (
                    usize::try_from(span.start.0).unwrap_or(usize::MAX),
                    usize::try_from(span.end.0).unwrap_or(usize::MAX),
                )
            } else {
                // SPIKE (ryo-fmx3.1, fault F): the catalog's text is a token's exact text and nothing for a node.
                (0, 0)
            };
            // A span of a file that is valid UTF-8 is a slice of it when it starts and ends between characters.
            let word = match names.source.and_then(|source| source.get(start..end)) {
                Some(text) => intern(words, NameText(text)),
                None => {
                    let bytes = names.text.0.get(start..end).unwrap_or_default();
                    intern(words, NameText(&String::from_utf8_lossy(bytes)))
                }
            };
            if let Some(held) = texts.get_mut(place) {
                *held = word;
            }
            word
        }
    }
}

/// `Read` over the node table: for each handle of the batch, the cell of the element's column. One loop for each column.
pub(crate) fn node_column(
    nodes: &NodeTable,
    names: NodeNames<'_>,
    words: &mut FileWords,
    texts: &mut Vec<CellWord>,
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
                out.push(node_cell(nodes, names, words, texts, column, *each));
            }
        }
    }
}

/// The word of a kind by its code: the cell the `Kind` column holds for an element of that kind.
#[inline]
pub(crate) fn kind_word(names: NodeNames<'_>, code: Count) -> CellWord {
    named(names.kinds, code)
}

/// The word of a field by its code: the cell the `Field` column holds for an element in that field.
#[inline]
pub(crate) fn field_word(names: NodeNames<'_>, code: Count) -> CellWord {
    named(names.fields, code)
}

/// One column of the node table as cells, element after element up to before `end`: what an index over the node table
/// is built from.
pub(crate) fn node_cells(
    nodes: &NodeTable,
    names: NodeNames<'_>,
    words: &mut FileWords,
    texts: &mut Vec<CellWord>,
    column: NodeColumn,
    end: Position,
    out: &mut Vec<CellWord>,
) {
    let tag = node_tag(names.file);
    out.clear();
    out.extend((0..end.0).map(|at| {
        node_cell(
            nodes,
            names,
            words,
            texts,
            column,
            tagged(CellWord(u64::from(at)), tag),
        )
    }));
}
