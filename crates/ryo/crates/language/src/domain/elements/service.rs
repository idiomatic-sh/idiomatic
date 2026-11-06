// the table builder and the readers of one element table are one vocabulary over the private `ElementsEngine` and `WrittenColumn` they reach through `AsRef`/`AsMut`, and nearly every function is a path the SDK hands languages (named in the product's tastemaker.toml); splitting would split the builder from what it builds and move SDK paths.
//! The `elements` service: free functions named by verb over a file's element table, with no IO
//! and no ports. An element is a node or a token, a row of the table.

use crate::domain::symbols::types::ByteSlice;

use super::constants::FOREIGN_POSITION;
use super::types::Diagnostic;
use crate::domain::elements::types::{
    BuilderEngine, Children, DecodedProperty, DiagnosticCode, DiagnosticMetadata, DiagnosticPhase,
    ElementPlacement, Elements, ElementsEngine, KindSum, ModelReadiness, Numbers, OpenNode, Parent,
    SourceOrder, TableBuilder, TableWriting, TreeImpact,
};
use crate::domain::symbols::service::{clear_symbols, intern, intern_derived};
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::constants::{IMPLIED, NO_FIELD, NO_SPAN, NO_SYMBOL, TOKEN};
use ryo_relations::domain::relations::types::ReportText;
use ryo_relations::domain::relations::types::{
    Bytes, Count, ElementOrigin, Extent, FieldId, Flags, KindBits, Offset, Position, PropertyId,
};
use ryo_relations::domain::terms::types::ChildOrdinal;
use ryo_relations::domain::terms::types::NameText;
use ryo_relations::domain::terms::types::Symbol;

/// Empties an element table, keeping its room for the next file.
pub fn clear_elements(elements: &mut Elements) {
    // Invalidate before touching the old population; stale models never survive a refill.
    engine_mut(elements).model_preparation.ready = ModelReadiness::Stale;
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
        engine.decoded_intern_visits = 0;
    }
    engine.decoded_text.0.clear();
    engine.names.clear();
    engine.vocabulary = KindSum(0);
    engine.kind_start.clear();
    engine.kind_order.clear();
    engine.open.clear();
    engine.notes.0.clear();
    engine.diagnostics.text.0.clear();
    engine.diagnostics.from.clear();
    engine.diagnostics.to.clear();
    engine.diagnostics.start.clear();
    engine.diagnostics.end.clear();
    engine.diagnostics.code.clear();
    engine.diagnostics.phase.clear();
    engine.diagnostics.impact.clear();
    engine.expanded_from = None;
    engine.syntax_acceptance = super::types::SyntaxAcceptance::Undetermined;
}

/// Record whether the language parser accepted the current source.
pub fn set_syntax_acceptance(
    builder: &mut TableBuilder<'_>,
    acceptance: super::types::SyntaxAcceptance,
) {
    builder_engine(builder)
        .elements
        .state
        .as_mut()
        .syntax_acceptance = acceptance;
}

/// The parser's acceptance decision, conservative for providers that did not supply one.
#[must_use]
pub fn syntax_acceptance(elements: &Elements) -> super::types::SyntaxAcceptance {
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

/// What `builder` writes through: the table, the names, the file's bytes and whether it writes.
pub(crate) fn builder_engine<'b, 'a>(
    builder: &'b mut TableBuilder<'a>,
) -> &'b mut BuilderEngine<'a> {
    builder.as_mut()
}

/// The identity of the next element the builder will append.
#[must_use]
pub fn builder_position(builder: &mut TableBuilder<'_>) -> Position {
    end_of(builder_engine(builder).elements)
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

/// Where the next element goes: its parent and its place among the parent's children.
fn place(elements: &mut Elements) -> ElementPlacement {
    match engine_mut(elements).open.last_mut() {
        Some(top) => {
            let ordinal = top.next;
            top.next = ChildOrdinal(ordinal.0 + 1);
            ElementPlacement {
                parent: Parent::Of(top.position),
                ordinal,
            }
        }
        None => ElementPlacement {
            parent: Parent::Root,
            ordinal: ChildOrdinal(0),
        },
    }
}

/// Appends an element below the innermost open node, and answers its position: for the callers that open it or mark it.
fn pushed(
    elements: &mut Elements,
    kind: KindBits,
    flags: Flags,
    start: Offset,
    end: Offset,
    text: Symbol,
) -> Position {
    let position = end_of(elements);
    push(elements, kind, flags, start, end, text);
    position
}

/// Appends an element below the innermost open node.
fn push(
    elements: &mut Elements,
    kind: KindBits,
    flags: Flags,
    start: Offset,
    end: Offset,
    text: Symbol,
) {
    let position = end_of(elements);
    let placement = place(elements);
    elements.kind.as_mut().values.push(kind);
    elements.flags.as_mut().values.push(flags);
    // Adopted expansion rows are hidden from the native tree's searchable columns.
    if engine(elements).expanded_from.is_none()
        && elements
            .start
            .as_ref()
            .values
            .last()
            .is_some_and(|previous| *previous > start)
    {
        engine_mut(elements).source_order = SourceOrder::Reordered;
    }
    elements.start.as_mut().values.push(start);
    elements.end.as_mut().values.push(end);
    elements.size.as_mut().values.push(Extent(0));
    let parent = match placement.parent {
        Parent::Of(parent) => parent,
        Parent::Root => position,
    };
    elements.parent.as_mut().values.push(parent);
    elements.ordinal.as_mut().values.push(placement.ordinal);
    elements.text.as_mut().values.push(text);
}

/// A builder that builds nothing, over the file whose bytes are `source`: what the engine reads a file through to know what the
/// library alone costs. The table and the names it is given are not touched.
pub fn discarding_builder<'a>(
    elements: &'a mut Elements,
    symbols: &'a mut Symbols,
    source: ByteSlice<'a>,
) -> TableBuilder<'a> {
    TableBuilder::from(BuilderEngine {
        elements,
        symbols,
        source,
        logical_source: None,
        writing: TableWriting::Discards,
    })
}

/// Opens a node of `kind` that starts at `start`; its children follow until [`leave_node`].
pub fn enter_node(builder: &mut TableBuilder<'_>, kind: KindBits, start: Offset) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let position = pushed(builder.elements, kind, Flags(0), start, start, NO_SYMBOL);
    engine_mut(builder.elements).open.push(OpenNode {
        position,
        next: ChildOrdinal(0),
    });
}

/// Closes the innermost open node, which ends at `end`, and records how many elements are below it.
pub fn leave_node(builder: &mut TableBuilder<'_>, end: Offset) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let elements = &mut *builder.elements;
    if let Some(open) = engine_mut(elements).open.pop() {
        let below = Extent(end_of(elements).0 - open.position.0 - 1);
        *number_mut(&mut elements.size, open.position) = below;
        if flags_at(elements, open.position).0 & IMPLIED.0 == 0 {
            *number_mut(&mut elements.end, open.position) = end;
        }
    }
}

/// Appends a token of `kind` between `start` and `end` of the file's bytes, and interns its text.
pub fn token(builder: &mut TableBuilder<'_>, kind: KindBits, start: Offset, end: Offset) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let text = token_name(builder, start, end);
    push(builder.elements, kind, TOKEN, start, end, text);
}

/// Appends a token of `kind` between `start` and `end` of the file's bytes, and interns its text and the text `decoded` it stands for
/// that the language's library read from it: what readers of the token read instead of decoding it again.
pub fn decoded_token(
    builder: &mut TableBuilder<'_>,
    kind: KindBits,
    start: Offset,
    end: Offset,
    decoded: NameText<'_>,
) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let text = token_name(builder, start, end);
    let element = pushed(builder.elements, kind, TOKEN, start, end, text);
    let table = engine_mut(builder.elements);
    let texts = &mut table.decoded_text;
    let from = Offset(u32::try_from(texts.0.len()).unwrap_or(u32::MAX));
    texts.0.extend_from_slice(decoded.0.as_bytes());
    let to = Offset(u32::try_from(texts.0.len()).unwrap_or(u32::MAX));
    table.decoded.push(DecodedProperty {
        element,
        property: PropertyId(0),
        from,
        to,
    });
}

/// Logical embedded text must outlive the temporary parser buffer as a derived name.
fn token_name(builder: &mut BuilderEngine<'_>, start: Offset, end: Offset) -> Symbol {
    if let Some(source) = builder.logical_source {
        let bytes = source
            .0
            .get(
                usize::try_from(start.0).unwrap_or(usize::MAX)
                    ..usize::try_from(end.0).unwrap_or(usize::MAX),
            )
            .unwrap_or_default();
        let text = std::str::from_utf8(bytes).unwrap_or_default();
        intern_derived(builder.symbols, builder.source, NameText(text))
    } else {
        intern(builder.symbols, builder.source, start, end)
    }
}

/// The text the token at `at` stands for, when the language's library decoded it.
#[must_use]
pub fn decoded_at(elements: &Elements, at: Position) -> Option<NameText<'_>> {
    property_at(elements, at, PropertyId(0))
}

/// A scalar parser property stored on this node, without claiming a source location.
#[must_use]
pub fn property_at(
    elements: &Elements,
    at: Position,
    property: PropertyId,
) -> Option<NameText<'_>> {
    let table = engine(elements);
    let Ok(found) = table
        .decoded
        .binary_search_by_key(&(at, property), |each| (each.element, each.property))
    else {
        return None; // a binary search answers where a key it does not find would go: that is the key's absence, not a failure
    };
    let each = table.decoded.get(found)?;
    table
        .decoded_text
        .0
        .get(
            usize::try_from(each.from.0).unwrap_or(usize::MAX)
                ..usize::try_from(each.to.0).unwrap_or(usize::MAX),
        )
        // ignore: idiomatic.arch/no-ok-on-result - the bytes are cut from text at the places the text was written: a cut that is no text has no text, which the caller reads as no value
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(NameText)
}

/// Records an untyped parser diagnostic with conservative incomplete-tree defaults.
pub fn diagnose(
    builder: &mut TableBuilder<'_>,
    message: ReportText<'_>,
    start: Offset,
    end: Offset,
) {
    diagnose_with_metadata(
        builder,
        DiagnosticMetadata::new(
            DiagnosticPhase::Parse,
            TreeImpact::Incomplete,
            DiagnosticCode::UNSPECIFIED,
        ),
        message,
        start,
        end,
    );
}

/// Records a diagnostic with its producer stage, stable provider code, and semantic tree impact.
pub fn diagnose_with_metadata(
    builder: &mut TableBuilder<'_>,
    metadata: DiagnosticMetadata,
    message: ReportText<'_>,
    start: Offset,
    end: Offset,
) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let diagnostics = &mut engine_mut(builder.elements).diagnostics;
    diagnostics.from.push(Offset(
        u32::try_from(diagnostics.text.0.len()).unwrap_or(u32::MAX),
    ));
    diagnostics.text.0.extend_from_slice(message.0.as_bytes());
    diagnostics.to.push(Offset(
        u32::try_from(diagnostics.text.0.len()).unwrap_or(u32::MAX),
    ));
    diagnostics.start.push(start);
    diagnostics.end.push(end);
    diagnostics.code.push(metadata.code);
    diagnostics.phase.push(metadata.phase);
    diagnostics.impact.push(metadata.impact);
}

/// The builder of the table of the file whose bytes are `source`; the table is cleared first.
pub fn table_builder<'a>(
    elements: &'a mut Elements,
    symbols: &'a mut Symbols,
    source: ByteSlice<'a>,
) -> TableBuilder<'a> {
    clear_elements(elements);
    clear_symbols(symbols);
    // A file has about two elements for five bytes: room for them is made once, not by doubling as they come.
    let expected = source.0.len() / 2;
    elements.start.as_mut().values.reserve(expected);
    elements.end.as_mut().values.reserve(expected);
    elements.size.as_mut().values.reserve(expected);
    elements.parent.as_mut().values.reserve(expected);
    elements.ordinal.as_mut().values.reserve(expected);
    elements.kind.as_mut().values.reserve(expected);
    elements.flags.as_mut().values.reserve(expected);
    elements.text.as_mut().values.reserve(expected);
    TableBuilder::from(BuilderEngine {
        elements,
        symbols,
        source,
        logical_source: None,
        writing: TableWriting::Writes,
    })
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

/// Everything the parser reported about the file, in the order it reported it.
pub fn diagnostics(elements: &Elements) -> impl Iterator<Item = Diagnostic<'_>> {
    let found = &engine(elements).diagnostics;
    found
        .from
        .iter()
        .zip(&found.to)
        .enumerate()
        .map(|(number, (from, to))| Diagnostic {
            message: ByteSlice(
                found
                    .text
                    .0
                    .get(
                        usize::try_from(from.0).unwrap_or(usize::MAX)
                            ..usize::try_from(to.0).unwrap_or(usize::MAX),
                    )
                    .unwrap_or(&[]),
            ),
            start: found.start.get(number).copied().unwrap_or_default(),
            end: found.end.get(number).copied().unwrap_or_default(),
            code: found
                .code
                .get(number)
                .copied()
                .unwrap_or(DiagnosticCode::UNSPECIFIED),
            phase: found
                .phase
                .get(number)
                .copied()
                .unwrap_or(DiagnosticPhase::Parse),
            impact: found
                .impact
                .get(number)
                .copied()
                .unwrap_or(TreeImpact::Incomplete),
        })
}

/// Whether any diagnostic indicates missing or recovered syntax.
#[must_use]
pub fn has_incomplete_syntax(elements: &Elements) -> bool {
    let diagnostics = &engine(elements).diagnostics;
    diagnostics.from.iter().enumerate().any(|(index, _)| {
        diagnostics
            .impact
            .get(index)
            .is_none_or(|impact| *impact != TreeImpact::Preserved)
    })
}

/// Whether lexical facts may be incomplete because tokenization failed or stopped early.
/// Missing impact metadata remains conservative: older providers do not certify lexical completeness.
#[must_use]
pub fn has_incomplete_lexical_stream(elements: &Elements) -> bool {
    let diagnostics = &engine(elements).diagnostics;
    diagnostics.from.iter().enumerate().any(|(index, _)| {
        diagnostics
            .impact
            .get(index)
            .is_none_or(|impact| *impact == TreeImpact::Incomplete)
    })
}

/// How many parse and validation diagnostics the provider reported about the file.
#[must_use]
pub fn diagnostic_count(elements: &Elements) -> Count {
    Count(engine(elements).diagnostics.from.len())
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
    // ignore: idiomatic.arch/no-unwrap-or-on-lookup - a file names only the roles it writes: a role it did not name has no symbol, as the doc says
    engine(elements)
        .field_names
        .get(usize::from(field_at(elements, at).0))
        .copied()
        .unwrap_or(NO_SYMBOL)
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
    expanded
        .binary_search_by(|each| each.element.cmp(&at))
        // ignore: idiomatic.arch/no-ok-on-result - a binary search answers where an element it does not find would go: that is the element's absence, not a failure
        .ok()
        .and_then(|found| expanded.get(found))
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
) -> Option<ryo_relations::domain::relations::types::EmbeddedSource> {
    let sources = &engine(elements).embedded;
    let next = sources.partition_point(|source| source.root <= at);
    next.checked_sub(1)
        .and_then(|index| sources.get(index))
        .filter(|source| at < source.end)
        .copied()
}

/// Opens a node the parser created without a source location.
pub fn enter_implied_node(builder: &mut TableBuilder<'_>, kind: KindBits) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let position = pushed(
        builder.elements,
        kind,
        IMPLIED,
        NO_SPAN.start,
        NO_SPAN.end,
        NO_SYMBOL,
    );
    engine_mut(builder.elements).open.push(OpenNode {
        position,
        next: ChildOrdinal(0),
    });
}

/// Records a scalar parser property on the open node. Key zero is its published decoded value.
/// Replacing a key replaces its value; properties do not add elements or source ranges.
pub fn node_property(builder: &mut TableBuilder<'_>, property: PropertyId, value: NameText<'_>) {
    // Formatting a borrowed string into retained memory cannot fail.
    let written = node_property_fmt(builder, property, format_args!("{}", value.0));
    debug_assert!(
        written.is_ok(),
        "a borrowed string formats into retained storage without failure"
    );
}

/// Format scalar parser metadata directly into retained storage, without a temporary string.
/// A formatter failure leaves both the property and its stored bytes unchanged.
///
/// # Errors
/// Returns the error of a supplied formatter; the storage writer itself is infallible.
pub fn node_property_fmt(
    builder: &mut TableBuilder<'_>,
    property: PropertyId,
    value: std::fmt::Arguments<'_>,
) -> std::fmt::Result {
    use std::fmt::Write as _;
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return Ok(());
    }
    let table = engine_mut(builder.elements);
    let Some(open) = table.open.last() else {
        return Ok(());
    };
    let element = open.position;
    let texts = &mut table.decoded_text.0;
    let checkpoint = texts.len();
    if let Err(error) = super::types::PropertyText(texts).write_fmt(value) {
        texts.truncate(checkpoint);
        return Err(error);
    }
    let record = DecodedProperty {
        element,
        property,
        from: Offset(u32::try_from(checkpoint).unwrap_or(u32::MAX)),
        to: Offset(u32::try_from(texts.len()).unwrap_or(u32::MAX)),
    };
    let records = &mut table.decoded;
    match records.binary_search_by_key(&(element, property), |each| (each.element, each.property)) {
        Ok(at) => {
            if let Some(slot) = records.get_mut(at) {
                *slot = record;
            }
        }
        Err(at) => records.insert(at, record),
    }
    Ok(())
}

/// Whether tree preorder can be searched by monotonically increasing source starts.
#[must_use]
pub fn source_order(elements: &Elements) -> SourceOrder {
    engine(elements).source_order
}
