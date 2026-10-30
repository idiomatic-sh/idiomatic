//! The `expansions` service: free functions named by verb that add the trees a language reads under a macro call, and mark the elements they make, with no IO and no ports.

use crate::domain::elements::service::{builder_engine, children, end_of, engine, engine_mut};
use crate::domain::elements::types::{OpenNode, TableBuilder, TableWriting};
use crate::domain::relations::types::Expanded;
use crate::domain::relations::types::{Count, ExpansionId, Position};
use crate::domain::terms::types::ChildOrdinal;

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
