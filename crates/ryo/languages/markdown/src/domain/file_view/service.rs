//! The `file_view` service: free functions over a walked file's syntax: what an element is, where it is, what the library decoded of
//! it, and a text as a row's cell.

use super::types::{MarkdownFile, Modelled, Syntax};
use crate::domain::extras::types::Extra;
use crate::domain::model::types::Sections;
use crate::domain::syntax::service::kind_of;
use crate::domain::syntax::types::{Kind, Level};
use ryo::languages::{self, NameText, NodeId, Offset, Position, Span, Symbol, node_table};

/// The kind of the element `at`.
#[must_use]
pub(crate) fn kind(syntax: &Syntax<'_>, at: Position) -> Kind {
    kind_of(node_table::kind_of(syntax.table, at))
}

/// The elements directly below `at`, in order.
pub(crate) fn children<'f>(
    syntax: &Syntax<'f>,
    at: Position,
) -> impl Iterator<Item = Position> + use<'f> {
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

/// What the library read from the element `at`: the code of a span, the label of a definition, the target a marker carries.
#[must_use]
pub(crate) fn decoded<'f>(syntax: &Syntax<'f>, at: Position) -> Option<&'f str> {
    node_table::decoded_of(syntax.table, at).map(|text| text.0)
}

/// The text the first marker of the construct `at` carries: the target of a link or an image, the label of a footnote, the depth of
/// a heading.
#[must_use]
pub(crate) fn named_by<'f>(syntax: &Syntax<'f>, at: Position) -> Option<&'f str> {
    let marker = children(syntax, at).find(|child| kind(syntax, *child) == Kind::Extra(Extra::Markup))?;
    decoded(syntax, marker)
}

/// How deep the heading `at` is; none for any other element and for a heading whose depth the library writes as no number.
#[must_use]
pub(crate) fn level_of(syntax: &Syntax<'_>, at: Position) -> Option<Level> {
    if kind(syntax, at) != Kind::Node(crate::domain::nodes::types::Node::Heading) {
        return None;
    }
    named_by(syntax, at)
        .and_then(|marked| marked.parse().ok())
        .map(Level)
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

/// The sections of the file, worked out by the first call, the file's syntax, and the text a filler builds for one row.
pub(crate) fn sectioned<'a>(
    file: &'a mut MarkdownFile<'_>,
) -> (&'a Sections, Syntax<'a>, &'a mut String) {
    let syntax = file.syntax;
    if file.modelled == Modelled::Waiting {
        crate::domain::walked_model::service::settle(&syntax, file.sections);
        file.modelled = Modelled::Done;
    }
    (&*file.sections, syntax, &mut *file.text)
}
