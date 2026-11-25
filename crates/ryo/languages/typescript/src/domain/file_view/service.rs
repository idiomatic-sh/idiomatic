//! The `file_view` service: free functions named by verb that read a walked file's numbered table for the fillers.

use super::types::TypeScriptFile;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, NameText, NodeId, Position, Span, Symbol, node_table};

/// The elements of one of the `kinds`, in the order of the table, each kind's after the last's.
pub(crate) fn of_kinds<'f>(
    file: &'f TypeScriptFile<'_>,
    kinds: &'f [Kind],
) -> impl Iterator<Item = Position> + 'f {
    kinds
        .iter()
        .flat_map(move |kind| node_table::ids_of_kind(file.table, kind_bits(*kind)).iter().copied())
}

/// The text the element `at` spans, as it is written; empty where the span is no text of the file.
#[must_use]
pub(crate) fn written<'f>(file: &TypeScriptFile<'f>, at: Position) -> NameText<'f> {
    let span = node_table::span_of(file.table, at);
    let from = usize::try_from(span.start.0).unwrap_or(usize::MAX);
    let to = usize::try_from(span.end.0).unwrap_or(usize::MAX);
    NameText(file.source.0.get(from..to).map(|bytes| std::str::from_utf8(bytes).unwrap_or_default()).unwrap_or_default())
}

/// The element as a row's node cell.
#[must_use]
pub(crate) fn node_cell(file: &TypeScriptFile<'_>, at: Position) -> NodeId {
    languages::node_id(file.file, at)
}

/// Where the element is, as a row's span.
#[must_use]
pub(crate) fn span_at(file: &TypeScriptFile<'_>, at: Position) -> Span {
    let span = node_table::span_of(file.table, at);
    Span { start: span.start, end: span.end }
}

/// A text as a row's text cell: a word of the file.
#[must_use]
pub(crate) fn word(words: &mut languages::FileWords, text: NameText<'_>) -> Symbol {
    Symbol(u32::try_from(languages::facts::intern(words, text).0).unwrap_or(u32::MAX))
}
