//! The `expansions` service: free functions named by verb that add the trees a language reads under a macro call, and mark the elements they make, with no IO and no ports.

use crate::domain::builders::service::builder_engine;
use crate::domain::elements::service::{children, end_of, engine, engine_mut};
use crate::domain::elements::types::{BuilderEngine, Elements};
use crate::domain::elements::types::{OpenNode, TableBuilder, TableWriting};
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::Expanded;
use ryo_relations::domain::relations::types::{Count, ExpansionId, Position};
use ryo_relations::domain::relations::types::{EmbeddedMapping, EmbeddedSource, Offset};
use ryo_relations::domain::terms::types::ChildOrdinal;

/// Adds trees for reading under the element at `host`, which the parser already closed: until [`release`], what is entered or added
/// is a child of `host` that comes after all its own, and is not an element of the file's own tree.
pub fn adopt(builder: &mut TableBuilder<'_>, host: Position) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let elements = &mut *builder.elements;
    let at = end_of(elements);
    let _: &mut Position = engine_mut(elements).expanded_from.get_or_insert(at);
    let next = children(elements, host).count();
    engine_mut(elements).open.push(OpenNode {
        position: host,
        next: ChildOrdinal(u32::try_from(next).unwrap_or(u32::MAX)),
    });
}

/// Ends what [`adopt`] began.
pub fn release(builder: &mut TableBuilder<'_>) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    let elements = &mut *builder.elements;
    let _: Option<OpenNode> = engine_mut(elements).open.pop();
    // What was added since the first adoption is not counted as the file's own.
    let added = engine(elements)
        .expanded_from
        .map_or(0, |from| end_of(elements).0.saturating_sub(from.0));
    let added = Count(usize::try_from(added).unwrap_or(usize::MAX));
    for hidden in [
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
        *hidden = added;
    }
}

/// Marks the element at `element` as one a macro expansion produced, under the call at `call_site`. A provider that expands macros
/// calls it; an element no provider marks is the file's own.
pub fn mark_expansion(
    builder: &mut TableBuilder<'_>,
    element: Position,
    call_site: Position,
    expansion: ExpansionId,
) {
    let builder = builder_engine(builder);
    if builder.writing == TableWriting::Discards {
        return;
    }
    engine_mut(builder.elements).expanded.push(Expanded {
        element,
        call_site,
        expansion,
    });
}

/// Parses logical text under a closed source carrier without changing the native tree.
/// The callback writes logical offsets and token text. After it returns, only source
/// intervals are mapped; interned text, parent edges, and syntax identities are retained.
/// Embedded elements are not marked as macro expansions.
pub fn write_embedded(
    builder: &mut TableBuilder<'_>,
    host: Position,
    source: ByteSlice<'_>,
    mapping: EmbeddedMapping,
    write: impl FnOnce(&mut TableBuilder<'_>),
) {
    if builder_engine(builder).writing == TableWriting::Discards {
        let engine = builder_engine(builder);
        let mut logical = TableBuilder::from(BuilderEngine {
            elements: engine.elements,
            symbols: engine.symbols,
            source: engine.source,
            logical_source: Some(source),
            writing: TableWriting::Discards,
        });
        write(&mut logical);
        return;
    }
    adopt(builder, host);
    let engine = builder_engine(builder);
    let root = end_of(engine.elements);
    let host_start = crate::domain::elements::service::start_at(engine.elements, host);
    let host_end = crate::domain::elements::service::end_at(engine.elements, host);
    {
        let mut logical = TableBuilder::from(BuilderEngine {
            elements: engine.elements,
            symbols: engine.symbols,
            source: engine.source,
            logical_source: Some(source),
            writing: engine.writing,
        });
        write(&mut logical);
    }
    let end = end_of(engine.elements);
    for position in root.0..end.0 {
        let index = usize::try_from(position).unwrap_or(usize::MAX);
        if let (Some(start), Some(span_end)) = (
            engine.elements.start.as_mut().values.get_mut(index),
            engine.elements.end.as_mut().values.get_mut(index),
        ) {
            match mapping {
                EmbeddedMapping::Exact(base) => {
                    *start = Offset(start.0.saturating_add(base.0));
                    *span_end = Offset(span_end.0.saturating_add(base.0));
                }
                EmbeddedMapping::WholeHost => {
                    *start = host_start;
                    *span_end = host_end;
                }
            }
        }
    }
    if root != end {
        engine_mut(engine.elements).embedded.push(EmbeddedSource {
            host,
            root,
            end,
            mapping,
        });
    }
    release(builder);
}

/// The independently parsed expressions retained by a file, in append order.
pub fn embedded_expressions(elements: &Elements) -> impl Iterator<Item = EmbeddedSource> + '_ {
    engine(elements).embedded.iter().copied()
}

/// Append through the existing embedding builder without clearing the native tree.
/// Decoded values written by this append are interned exactly once afterward.
pub fn append_embedded(
    completion: &mut crate::domain::elements::types::Completion<'_>,
    write: impl FnOnce(&mut TableBuilder<'_>),
) {
    let completion = completion.as_mut();
    let from = Count(engine(completion.elements).decoded.len());
    {
        let mut builder = TableBuilder::from(BuilderEngine {
            elements: completion.elements,
            symbols: completion.symbols,
            source: completion.source,
            logical_source: None,
            writing: TableWriting::Writes,
        });
        write(&mut builder);
    }
    crate::domain::completion::service::intern_decoded_from(
        completion.elements,
        completion.symbols,
        completion.source,
        from,
    );
}
