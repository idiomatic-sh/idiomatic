//! The `file_view` service: free functions over a walked file's syntax: what an element is, where it is, what the library decoded
//! of it, and a text as a row's cell.

use super::types::{JsonFile, Syntax};
use crate::domain::model::types::Model;
use crate::domain::syntax::service::kind_of;
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, NameText, NodeId, Offset, Position, Span, Symbol, node_table};

/// The kind of the element `at`.
#[must_use]
pub(crate) fn kind(syntax: &Syntax<'_>, at: Position) -> Kind {
    kind_of(node_table::kind_of(syntax.table, at))
}

/// The elements directly below `at`, in order.
pub(crate) fn children<'f>(syntax: &Syntax<'f>, at: Position) -> impl Iterator<Item = Position> + use<'f> {
    let table: &'f languages::NodeTable = syntax.table;
    node_table::children_of(table, at).iter().copied()
}

/// The element `at` is in; the root is its own.
#[must_use]
pub(crate) fn parent(syntax: &Syntax<'_>, at: Position) -> Position {
    node_table::parent_of(syntax.table, at)
}

/// The text between two offsets of the file, as it is written; empty where the offsets are no text of the file.
#[must_use]
pub(crate) fn between<'f>(syntax: &Syntax<'f>, start: Offset, end: Offset) -> &'f str {
    let (from, to) = (
        usize::try_from(start.0).unwrap_or(usize::MAX),
        usize::try_from(end.0).unwrap_or(usize::MAX),
    );
    // the bytes of an element that are not text have no text, which a row takes as an empty name: the row keeps its place either way
    syntax
        .text
        .get(from..to)
        .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
        .unwrap_or_default()
}

/// The text the element `at` spans, as it is written.
#[must_use]
pub(crate) fn written<'f>(syntax: &Syntax<'f>, at: Position) -> &'f str {
    let span = node_table::span_of(syntax.table, at);
    between(syntax, span.start, span.end)
}

/// What the library decoded of the token `at`; nothing for a token it did not decode, or refused.
#[must_use]
pub(crate) fn decoded<'f>(syntax: &Syntax<'f>, at: Position) -> Option<&'f str> {
    let span = node_table::span_of(syntax.table, at);
    let (from, to) = *syntax.decodings.places.get(&(span.start.0, span.end.0))?;
    syntax.decodings.text.get(from..to)
}

/// Where an element is, as a row's span.
#[must_use]
pub(crate) fn span_at(syntax: &Syntax<'_>, at: Position) -> Span {
    let span = node_table::span_of(syntax.table, at);
    Span {
        start: span.start,
        end: span.end,
    }
}

/// An element as a row's node cell.
#[must_use]
pub(crate) fn node_cell(syntax: &Syntax<'_>, at: Position) -> NodeId {
    languages::node_id(syntax.file, at)
}

/// A text as a row's text cell: a word of the file.
pub(crate) fn word(words: &mut languages::FileWords, text: &str) -> Symbol {
    Symbol(u32::try_from(languages::facts::intern(words, NameText(text)).0).unwrap_or(u32::MAX))
}

/// The model of the file, worked out by the first call.
pub(crate) fn modelled<'a>(file: &'a mut JsonFile<'_>) -> (&'a Model, Syntax<'a>) {
    let syntax = file.syntax;
    if file.modelled == super::types::Modelled::Waiting {
        crate::domain::walked_values::service::places(&syntax, file.model);
        file.modelled = super::types::Modelled::Done;
    }
    (&*file.model, syntax)
}

/// Keeps what the library decoded of the token over `start..end`.
pub(crate) fn record(decodings: &mut super::types::Decodings, start: Offset, end: Offset, text: &str) {
    let from = decodings.text.len();
    decodings.text.push_str(text);
    let _ = decodings.places.insert((start.0, end.0), (from, decodings.text.len()));
}
