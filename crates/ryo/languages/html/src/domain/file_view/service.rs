//! The `file_view` service: free functions over a walked file: what an element is, where it is, what the library decoded of it, and a
//! text as a row's cell.

use super::types::HtmlFile;
use crate::domain::syntax::service::{kind_of, property_id};
use crate::domain::syntax::types::{Kind, Property};
use ryo::languages::{self, NameText, NodeId, Position, Span, Symbol, node_table};

/// The kind of the element `at`.
#[must_use]
pub(crate) fn kind(file: &HtmlFile<'_>, at: Position) -> Kind {
    kind_of(node_table::kind_of(file.table, at))
}

/// The elements directly below `at`, in order.
pub(crate) fn children<'f>(file: &HtmlFile<'f>, at: Position) -> impl Iterator<Item = Position> + use<'f> {
    let table: &'f languages::NodeTable = file.table;
    node_table::children_of(table, at).iter().copied()
}

/// The element `at` is in; the root is its own.
#[must_use]
pub(crate) fn parent(file: &HtmlFile<'_>, at: Position) -> Position {
    node_table::parent_of(file.table, at)
}

/// Every element of the file, in order.
pub(crate) fn elements(file: &HtmlFile<'_>) -> impl Iterator<Item = Position> + use<> {
    let count = u32::try_from(node_table::element_count(file.table).0).unwrap_or(u32::MAX);
    (0..count).map(Position)
}

/// A decoded DOM property of the element `at`: empty where it has none.
#[must_use]
pub(crate) fn property<'f>(file: &HtmlFile<'f>, at: Position, property: Property) -> &'f str {
    let table: &'f languages::NodeTable = file.table;
    node_table::property_of(table, at, property_id(property)).map_or("", |text| text.0)
}

/// The path of the run a URL-valued attribute points at, as the adapter wrote it on the attribute.
#[must_use]
pub(crate) fn target<'f>(file: &HtmlFile<'f>, at: Position) -> &'f str {
    let table: &'f languages::NodeTable = file.table;
    node_table::property_of(table, at, crate::domain::syntax::constants::TARGET_PROPERTY).map_or("", |text| text.0)
}

/// Where an element is, as a row's span.
#[must_use]
pub(crate) fn span_at(file: &HtmlFile<'_>, at: Position) -> Span {
    let span = node_table::span_of(file.table, at);
    Span { start: span.start, end: span.end }
}

/// An element as a row's node cell.
#[must_use]
pub(crate) fn node_cell(file: &HtmlFile<'_>, at: Position) -> NodeId {
    languages::node_id(file.file, at)
}

/// A text as a row's text cell: a word of the file.
pub(crate) fn word(words: &mut languages::FileWords, text: &str) -> Symbol {
    Symbol(u32::try_from(languages::facts::intern(words, NameText(text)).0).unwrap_or(u32::MAX))
}
