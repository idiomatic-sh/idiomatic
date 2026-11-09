//! The `elements` service: free functions named by verb over a file's element table, with no IO
//! and no ports. An element is a node or a token, a row of the table.

use super::constants::FOREIGN_POSITION;
use super::types::{
    Children, Elements, ElementsEngine, KindSum, ModelReadiness, Numbers, SourceOrder,
    SyntaxAcceptance,
};
use crate::domain::diagnostics::service::{
    clear, count, incomplete_lexical_stream, incomplete_syntax, listed,
};
use crate::domain::diagnostics::types::{Diagnostic, SyntaxCompleteness};
use ryo_relations::domain::elements::types::ElementOrigin;
use ryo_relations::domain::relations::constants::{IMPLIED, NO_FIELD, NO_SYMBOL, TOKEN};
use ryo_relations::domain::relations::types::FieldId;
use ryo_relations::domain::relations::types::{
    Bytes, Count, Extent, Flags, KindBits, Offset, Position,
};
use ryo_relations::domain::terms::types::ChildOrdinal;
use ryo_relations::domain::terms::types::Symbol;

/// What the language kept of its read of the file for a semantic session, taken: the table keeps nothing of it after.
pub fn take_semantics(elements: &mut Elements) -> crate::domain::semantics::types::SemanticSlot {
    engine_mut(elements).retained.0.take().map_or(
        crate::domain::semantics::types::SemanticSlot::Empty,
        crate::domain::semantics::types::SemanticSlot::Retained,
    )
}

/// Empties an element table, keeping its room for the next file.
pub fn clear_elements(elements: &mut Elements) {
    // Invalidate before touching the old population; stale models never survive a refill.
    engine_mut(elements).model_preparation.ready = ModelReadiness::Stale;
    engine_mut(elements).retained.0 = None;
    elements.kind.as_mut().values.clear();
    elements.flags.as_mut().values.clear();
    elements.start.as_mut().values.clear();
    elements.end.as_mut().values.clear();
    elements.size.as_mut().values.clear();
    elements.parent.as_mut().values.clear();
    elements.ordinal.as_mut().values.clear();
    elements.field.as_mut().values.clear();
    elements.text.as_mut().values.clear();
    for added in [
        &mut elements.kind.as_mut().added,
        &mut elements.flags.as_mut().added,
        &mut elements.start.as_mut().added,
        &mut elements.end.as_mut().added,
        &mut elements.size.as_mut().added,
        &mut elements.parent.as_mut().added,
        &mut elements.ordinal.as_mut().added,
        &mut elements.field.as_mut().added,
        &mut elements.text.as_mut().added,
    ] {
        *added = Count(0);
    }
    let engine = engine_mut(elements);
    engine.line_starts.0.clear();
    engine.source_order = SourceOrder::Forward;
    engine.field_names.clear();
    engine.expanded.clear();
    engine.embedded.clear();
    engine.decoded.clear();
    #[cfg(test)]
    {
        engine.decoded_intern_visits = Count(0);
    }
    engine.decoded_text.0.clear();
    engine.names.clear();
    engine.vocabulary = KindSum(0);
    engine.kind_start.clear();
    engine.kind_order.clear();
    engine.open.clear();
    engine.notes.0.clear();
    clear(&mut engine.diagnostics);
    engine.expanded_from = None;
    engine.syntax_acceptance = SyntaxAcceptance::Undetermined;
}

/// The parser's acceptance decision, conservative for providers that did not supply one.
#[must_use]
pub fn syntax_acceptance(elements: &Elements) -> SyntaxAcceptance {
    engine(elements).syntax_acceptance
}

/// What the engine keeps of `elements` beside its columns, to read.
pub(crate) fn engine(elements: &Elements) -> &ElementsEngine {
    elements.state.as_ref()
}

/// What the engine keeps of `elements` beside its columns, to write.
pub(crate) fn engine_mut(elements: &mut Elements) -> &mut ElementsEngine {
    elements.state.as_mut()
}

/// The position the next element takes: the table holds this many elements, those the language added for reading included.
#[must_use]
pub fn end_of(elements: &Elements) -> Position {
    Position(u32::try_from(elements.kind.as_ref().values.len()).unwrap_or(u32::MAX))
}

/// How many elements the file's own tree has: the table without the elements the language added for reading.
#[must_use]
pub fn real_count(elements: &Elements) -> Count {
    Count(elements.kind.len())
}

/// Every position of the file's own tree, in order.
pub fn positions(elements: &Elements) -> impl Iterator<Item = Position> + use<> {
    (0..u32::try_from(real_count(elements).0).unwrap_or(u32::MAX)).map(Position)
}

/// Every position of the elements the language added for reading, in order; none when it added none.
pub fn expanded_positions(elements: &Elements) -> impl Iterator<Item = Position> + use<> {
    let from = engine(elements)
        .expanded_from
        .unwrap_or_else(|| end_of(elements));
    (from.0..end_of(elements).0).map(Position)
}

/// The kind of the element at `position`, as the language numbers it.
#[must_use]
pub fn kind_at(elements: &Elements, position: Position) -> KindBits {
    number(&elements.kind, position)
}

/// Where the element at `position` starts.
#[must_use]
pub fn start_at(elements: &Elements, position: Position) -> Offset {
    number(&elements.start, position)
}

/// The number `column` holds for the element at `at`, the elements the language added for reading included.
///
/// # Panics
///
/// On a position the table never gave: that is a bug in the caller, and an empty element in its place would be a silently wrong
/// answer.
#[expect(
    clippy::expect_used,
    reason = "a position comes only from this table; another table's position is a bug that must crash, not answer an empty element"
)]
fn number<T>(column: &Numbers<T>, at: Position) -> T
where
    T: Copy,
{
    column
        .as_ref()
        .values
        .get(usize::try_from(at.0).unwrap_or(usize::MAX))
        .copied()
        .expect(FOREIGN_POSITION)
}

/// The number `column` holds for the element at `at`, to write.
///
/// # Panics
///
/// On a position the table never gave: a write there would be lost, and the table would answer as if it had been made.
#[expect(
    clippy::expect_used,
    reason = "a position comes only from this table; another table's position is a bug that must crash, not a write that is lost"
)]
pub(crate) fn number_mut<T>(column: &mut Numbers<T>, at: Position) -> &mut T {
    column
        .as_mut()
        .values
        .get_mut(usize::try_from(at.0).unwrap_or(usize::MAX))
        .expect(FOREIGN_POSITION)
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub fn parent_at(elements: &Elements, at: Position) -> Position {
    number(&elements.parent, at)
}

/// The bit flags of the element at `at`: [`TOKEN`] marks a token.
#[must_use]
pub fn flags_at(elements: &Elements, at: Position) -> Flags {
    number(&elements.flags, at)
}

/// How many elements are below the element at `at`: none below a token.
#[must_use]
pub fn size_at(elements: &Elements, at: Position) -> Extent {
    number(&elements.size, at)
}

/// The last element below the element at `at`, or `at` itself when nothing is below it.
#[must_use]
pub fn last_at(elements: &Elements, at: Position) -> Position {
    Position(at.0 + size_at(elements, at).0)
}

/// Where the element at `position` ends.
#[must_use]
pub fn end_at(elements: &Elements, position: Position) -> Offset {
    number(&elements.end, position)
}

/// The children of the element at `parent`, in order.
#[must_use]
pub fn children(elements: &Elements, parent: Position) -> Children<'_> {
    Children {
        elements,
        next: Position(parent.0 + 1),
        last: last_at(elements, parent),
    }
}

/// The child just before the element at `at` under its parent; none for a first child or the root.
#[must_use]
pub fn previous_sibling(elements: &Elements, at: Position) -> Option<Position> {
    if at.0 == 0 {
        return None;
    }
    children(elements, parent_at(elements, at))
        .take_while(|each| *each != at)
        .last()
}

/// The child just after the element at `at` under its parent; none for a last child or the root.
#[must_use]
pub fn next_sibling(elements: &Elements, at: Position) -> Option<Position> {
    if at.0 == 0 {
        return None;
    }
    children(elements, parent_at(elements, at))
        .skip_while(|each| *each != at)
        .nth(1)
}

/// The first child of the element at `of` whose kind is `kind`.
#[must_use]
pub fn child_of_kind(elements: &Elements, of: Position, kind: KindBits) -> Option<Position> {
    children(elements, of).find(|each| kind_at(elements, *each) == kind)
}

/// The text of the token at `at`, as the name it was interned as; no symbol for a node.
#[must_use]
pub fn text_at(elements: &Elements, at: Position) -> Symbol {
    number(&elements.text, at)
}

/// The text of the first token among the children of the element at `of`: the name a node that holds one holds.
#[must_use]
pub fn first_token_text(elements: &Elements, of: Position) -> Option<Symbol> {
    children(elements, of)
        .find(|each| flags_at(elements, *each).0 & TOKEN.0 != 0)
        .map(|each| text_at(elements, each))
}

/// The place of the element at `at` among its parent's children.
#[must_use]
pub fn ordinal_at(elements: &Elements, at: Position) -> ChildOrdinal {
    number(&elements.ordinal, at)
}

/// How many elements the table holds.
#[must_use]
pub fn element_count(elements: &Elements) -> Count {
    Count(elements.kind.len())
}

/// What the names of the kinds of the file add up to: the same for every file of a language, and not for another.
#[must_use]
pub fn kind_vocabulary(elements: &Elements) -> KindSum {
    engine(elements).vocabulary
}

/// What each kind is called, by the number of the kind, as the file's names; no symbol for a kind none of its elements has.
#[must_use]
pub fn kind_names(elements: &Elements) -> &[Symbol] {
    &engine(elements).names
}

/// What the language wrote of the file besides its tree, bytes only it reads.
#[must_use]
pub fn notes_of(elements: &Elements) -> &Bytes {
    &engine(elements).notes
}

/// Takes what the language wrote of the file besides its tree, leaving none.
pub fn take_notes(elements: &mut Elements) -> Bytes {
    std::mem::take(&mut engine_mut(elements).notes)
}

/// What the language wrote of the file besides its tree, to change: the engine tags it with the language that wrote it.
pub fn notes_mut(elements: &mut Elements) -> &mut Bytes {
    &mut engine_mut(elements).notes
}

/// The role of the element at `at` under its parent, as the language numbers its roles; no role while the roles are not written:
/// a language that names none never writes them, and a language reads the tree while its roles are being written.
#[must_use]
pub fn field_at(elements: &Elements, at: Position) -> FieldId {
    if elements.field.as_ref().values.is_empty() {
        NO_FIELD
    } else {
        number(&elements.field, at)
    }
}

/// The name of the role of the element at `at`, as the name the language gave the role in this file; no symbol for a role it did not name.
#[must_use]
pub fn field_name_at(elements: &Elements, at: Position) -> Symbol {
    // A file names only the roles it writes: a role it did not name has no symbol, as the doc says.
    match engine(elements)
        .field_names
        .get(usize::from(field_at(elements, at).0))
    {
        Some(symbol) => *symbol,
        None => NO_SYMBOL,
    }
}

/// Where the element at `at` comes from.
#[must_use]
pub fn origin_at(elements: &Elements, at: Position) -> ElementOrigin {
    if flags_at(elements, at).0 & IMPLIED.0 != 0 {
        return ElementOrigin::Implied;
    }
    if let Some(embedded) = embedded_source_at(elements, at) {
        return ElementOrigin::Embedded {
            host: embedded.host,
        };
    }
    let expanded = &engine(elements).expanded;
    // A binary search answers where an element it does not find would go: that is the element's absence, not a failure.
    match expanded.binary_search_by(|each| each.element.cmp(&at)) {
        Ok(found) => expanded.get(found),
        Err(_) => None,
    }
    .map_or(ElementOrigin::File, |mark| ElementOrigin::Expansion {
        call_site: mark.call_site,
        expansion: mark.expansion,
    })
}

/// The source carrier of one appended occurrence. Native syntax has no carrier.
#[must_use]
pub fn embedded_source_at(
    elements: &Elements,
    at: Position,
) -> Option<ryo_relations::domain::elements::types::EmbeddedSource> {
    holding(&engine(elements).embedded, at, |source| {
        source.root..source.end
    })
}

/// The carrier holding `at` among disjoint carriers ascending by root, found by bisection;
/// `bounds` reads one carrier's appended interval per comparison.
pub(crate) fn holding(
    sources: &[ryo_relations::domain::elements::types::EmbeddedSource],
    at: Position,
    mut bounds: impl FnMut(
        &ryo_relations::domain::elements::types::EmbeddedSource,
    ) -> core::ops::Range<Position>,
) -> Option<ryo_relations::domain::elements::types::EmbeddedSource> {
    let next = sources.partition_point(|source| bounds(source).start <= at);
    next.checked_sub(1)
        .and_then(|index| sources.get(index))
        .filter(|source| at < bounds(source).end)
        .copied()
}

/// Whether tree preorder can be searched by monotonically increasing source starts.
#[must_use]
pub fn source_order(elements: &Elements) -> SourceOrder {
    engine(elements).source_order
}

/// Everything the parser reported about the file, in the order it reported it.
pub fn diagnostics(elements: &Elements) -> impl Iterator<Item = Diagnostic<'_>> {
    listed(&engine(elements).diagnostics)
}

/// Whether any diagnostic indicates missing or recovered syntax.
#[must_use]
pub fn has_incomplete_syntax(elements: &Elements) -> SyntaxCompleteness {
    incomplete_syntax(&engine(elements).diagnostics)
}

/// Whether lexical facts may be incomplete because tokenization failed or stopped early.
/// Missing impact metadata remains conservative: older providers do not certify lexical completeness.
#[must_use]
pub fn has_incomplete_lexical_stream(elements: &Elements) -> SyntaxCompleteness {
    incomplete_lexical_stream(&engine(elements).diagnostics)
}

/// How many parse and validation diagnostics the provider reported about the file.
#[must_use]
pub fn diagnostic_count(elements: &Elements) -> Count {
    count(&engine(elements).diagnostics)
}
