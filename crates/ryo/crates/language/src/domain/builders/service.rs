//! The `builders` service: free functions named by verb that write a file's element table through a builder: open a node, add a token,
//! close a node, report what the parser found. A builder that discards writes nothing.

use super::types::{ElementPlacement, Parent, PropertyText};
use crate::domain::diagnostics::constants::UNSPECIFIED_CODE;
use crate::domain::diagnostics::service::record;
use crate::domain::diagnostics::types::{DiagnosticMetadata, DiagnosticPhase, TreeImpact};
use crate::domain::elements::service::{
    clear_elements, end_of, engine, engine_mut, flags_at, number_mut,
};
use crate::domain::elements::types::{
    BuilderEngine, DecodedProperty, Elements, OpenNode, SourceOrder, SyntaxAcceptance,
    TableBuilder, TableWriting,
};
use crate::domain::symbols::service::{clear_symbols, intern, intern_derived};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::constants::{IMPLIED, NO_SPAN, NO_SYMBOL, TOKEN};
use ryo_relations::domain::relations::types::ReportText;
use ryo_relations::domain::relations::types::{Extent, Flags, KindBits, Offset, Position};
use ryo_relations::domain::relations::types::PropertyId;
use ryo_relations::domain::terms::types::ChildOrdinal;
use ryo_relations::domain::terms::types::NameText;
use ryo_relations::domain::terms::types::Symbol;

/// Record whether the language parser accepted the current source.
pub fn set_syntax_acceptance(builder: &mut TableBuilder<'_>, acceptance: SyntaxAcceptance) {
    builder_engine(builder)
        .elements
        .state
        .as_mut()
        .syntax_acceptance = acceptance;
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

/// Records an untyped parser diagnostic with conservative incomplete-tree defaults.
pub fn diagnose(
    builder: &mut TableBuilder<'_>,
    message: ReportText<'_>,
    start: Offset,
    end: Offset,
) {
    diagnose_with_metadata(
        builder,
        DiagnosticMetadata {
            phase: DiagnosticPhase::Parse,
            impact: TreeImpact::Incomplete,
            code: UNSPECIFIED_CODE,
        },
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
    record(
        &mut engine_mut(builder.elements).diagnostics,
        metadata,
        message,
        start,
        end,
    );
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
    if let Err(error) = PropertyText(texts).write_fmt(value) {
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
