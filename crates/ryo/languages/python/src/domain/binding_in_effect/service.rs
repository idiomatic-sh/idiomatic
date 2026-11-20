//! Schedule-order lookup (Spike ryo-m1ea): run the file's schedule in order and, at each read or deletion, walk Python's lookup
//! path (Language Reference 4.2.2, 7.12, 7.13; PEP 695). In each namespace on the path, separately for each code block that stores
//! into it (its own block, and each block that declares the name `global` or `nonlocal`), give the binding in effect (the latest
//! bind before the reference that no ended exception handler encloses, 8.4) and each later event of the name; when no bind precedes
//! the reference, every event. Which later event unbinds the name for a reader is not decided here: a deletion in a branch may or
//! may not have run, and that choice belongs to the reader.
use super::types::{BindingLookupScratch, LookupRow, NONE};
use crate::domain::{
    binding_analysis::service::indexed,
    binding_resolution::{
        service::{context_of, in_header, key, lazy, root_of},
        types::TypeParameter,
    },
    binding_results::types::{BindingResultEvent, EventKind, ReadContext},
    definitions::service::definition,
    model::types::{BindingAnalysisIndex, BindingAnalysisState, Declarations},
    nodes::types::Node,
    scopes::service::{declares_global, enclosing},
    syntax::{
        service::{is_root, kind, parent},
        types::{Kind, Rootness},
    },
};
use ryo::languages::{self, Integer, Position, Symbols, Tree};
use std::cell::Ref;

/// The file's lookup, made on the first call for the file; `None` when the binding analysis is incomplete.
pub(crate) fn looked_up<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Option<Ref<'a, BindingLookupScratch>> {
    let index = indexed(tree, model, symbols)?;
    if model.binding_lookup.borrow().state == BindingAnalysisState::Fresh {
        let mut out = model.binding_lookup.borrow_mut();
        look_up(tree, model, &index, &mut out);
        out.state = BindingAnalysisState::Complete;
    }
    drop(index);
    Some(model.binding_lookup.borrow())
}

/// Forget the current file's lookup while keeping every buffer.
pub(crate) fn clear(out: &mut BindingLookupScratch) {
    out.state = BindingAnalysisState::Fresh;
    out.rows.clear();
    out.latest.clear();
    out.previous.clear();
    out.routed.clear();
    out.declared.clear();
    out.parameters.clear();
    out.lazy.clear();
    out.order.clear();
    out.sorting.clear();
    out.found.clear();
    out.ended.clear();
}

/// What one reference's walk reads: the tree, the model and the events.
struct Walk<'t, 'a> {
    tree: &'t Tree<'t>,
    model: &'a Declarations,
    index: &'a BindingAnalysisIndex,
}

/// The index of an event as a chain link.
fn link(at: usize) -> u32 {
    u32::try_from(at).unwrap_or(NONE)
}

/// One pass over the schedule's events; lazy reads are looked up against the state at the end of the schedule.
fn look_up(tree: &Tree<'_>, model: &Declarations, index: &BindingAnalysisIndex, out: &mut BindingLookupScratch) {
    clear(out);
    for at in languages::positions(tree.elements) {
        if !matches!(kind(tree, at), Kind::Node(Node::TypeVar | Node::TypeVarTuple | Node::ParamSpec)) {
            continue;
        }
        if let Some(defined) = definition(tree, model, at) {
            out.parameters.push(TypeParameter { owner: defined.owner, id: at, name: defined.name });
        }
    }
    out.previous.resize(index.events.len(), NONE);
    let walk = Walk { tree, model, index };
    // 7.12, 7.13: a global or nonlocal statement holds for its entire code block, so a declaring namespace holds its declaration
    // for every reference, even one the schedule reaches before the statement.
    for (at, event) in index.events.iter().enumerate() {
        if matches!(event.kind, EventKind::Global | EventKind::Nonlocal) && event.lexical != event.scope {
            let _first = out.declared.entry(key(event.lexical, event.name)).or_insert_with(|| link(at));
        }
    }
    let mut cursor = 0;
    for (at, event) in index.events.iter().enumerate() {
        match event.kind {
            EventKind::Read | EventKind::Delete => {
                let context = context_of(index, &mut cursor, event.occurrence, event.kind);
                if event.kind == EventKind::Read && lazy(tree, event.occurrence) {
                    out.lazy.push((at, context));
                } else {
                    walk.reference(at, context, out);
                }
                if event.kind == EventKind::Delete {
                    append(out, at, event);
                }
            }
            EventKind::Bind | EventKind::Annotation | EventKind::Cleanup | EventKind::Global | EventKind::Nonlocal => {
                append(out, at, event);
            }
            EventKind::Wildcard => {}
        }
    }
    if !out.lazy.is_empty() {
        let lazy = std::mem::take(&mut out.lazy);
        for (at, context) in &lazy {
            walk.reference(*at, *context, out);
        }
        out.lazy = lazy;
        let mut rows = std::mem::take(&mut out.sorting);
        rows.clear();
        rows.extend(out.order.iter().copied().zip(out.rows.iter().copied()));
        rows.sort_by_key(|(ordinal, _)| ordinal.0);
        out.rows.clear();
        out.order.clear();
        for (ordinal, row) in &rows {
            out.order.push(*ordinal);
            out.rows.push(*row);
        }
        out.sorting = rows;
    }
}

/// Add the event at `at` to the chain of its (namespace, name).
fn append(out: &mut BindingLookupScratch, at: usize, event: &BindingResultEvent) {
    let slot = key(event.scope, event.name);
    let before = out.latest.insert(slot, link(at)).unwrap_or(NONE);
    if let Some(previous) = out.previous.get_mut(at) {
        *previous = before;
    }
    if event.lexical != event.scope {
        let _new = out.routed.insert(slot);
    }
}

impl Walk<'_, '_> {
    /// Emit the rows of the reference at event index `at`: header type parameters, then the path from its own namespace.
    fn reference(&self, at: usize, context: ReadContext, out: &mut BindingLookupScratch) {
        let Some(event) = self.index.events.get(at).copied() else {
            return;
        };
        let tree = self.tree;
        let start = enclosing(tree, event.occurrence);
        let root = root_of(tree, start);
        let mut depth = 0_i64;
        if !out.parameters.is_empty() {
            let mut header = false;
            let mut above = event.occurrence;
            while is_root(tree, above) == Rootness::Inner {
                above = parent(tree, above);
                if out.parameters.iter().any(|each| each.owner == above) && in_header(tree, event.occurrence, above, start) {
                    header = true;
                    parameters(above, event, depth, context, out);
                }
            }
            if header {
                depth += 1;
            }
        }
        let mut scope = start;
        let mut first = true;
        loop {
            let skipped = !first && kind(tree, scope) == Kind::Node(Node::ClassDef);
            if !skipped {
                self.namespace(scope, event, depth, context, out);
                depth += 1;
                if scope != root && declares_global(tree, self.model, scope, event.name) {
                    self.namespace(root, event, depth, context, out);
                    break;
                }
            }
            if scope == root {
                break;
            }
            if out.parameters.iter().any(|each| each.owner == scope) {
                parameters(scope, event, depth, context, out);
                depth += 1;
            }
            scope = enclosing(tree, scope);
            first = false;
        }
    }

    /// The rows of the namespace `scope`: its declaration of the name, if it routes the name elsewhere, then, per code block that
    /// stores into it, the events back to and including the binding in effect.
    fn namespace(&self, scope: Position, reference: BindingResultEvent, depth: i64, context: ReadContext, out: &mut BindingLookupScratch) {
        let slot = key(scope, reference.name);
        if let Some(found) = out.declared.get(&slot).and_then(|at| self.index.events.get(*at as usize)).copied() {
            push(out, reference, &found, scope, depth, context);
        }
        let mut at = out.latest.get(&slot).copied().unwrap_or(NONE);
        if at == NONE {
            return;
        }
        let routed = out.routed.contains(&slot);
        out.found.clear();
        out.ended.clear();
        while at != NONE {
            let Some(event) = self.index.events.get(at as usize).copied() else {
                break;
            };
            at = out.previous.get(at as usize).copied().unwrap_or(NONE);
            if out.found.contains(&event.lexical) {
                continue;
            }
            // 8.4: a handler's target is unbound when the handler ends ("as if" a finally clause deleted it), so after the cleanup
            // no event of the name between the handler's own bind and the cleanup is in effect: they are passed over.
            if let Some(inside) = out.ended.iter().position(|(block, _)| *block == event.lexical) {
                if event.kind == EventKind::Bind && out.ended.get(inside).is_some_and(|(_, handler)| *handler == event.occurrence) {
                    let _left = out.ended.swap_remove(inside);
                }
                continue;
            }
            match event.kind {
                EventKind::Bind => {
                    push(out, reference, &event, scope, depth, context);
                    out.found.push(event.lexical);
                    if !routed {
                        break;
                    }
                }
                EventKind::Cleanup => {
                    out.ended.push((event.lexical, event.occurrence));
                    push(out, reference, &event, scope, depth, context);
                }
                EventKind::Annotation | EventKind::Delete | EventKind::Global | EventKind::Nonlocal => {
                    push(out, reference, &event, scope, depth, context);
                }
                EventKind::Read | EventKind::Wildcard => {}
            }
        }
    }
}

/// One row: the event `found` in the namespace `scope` for `reference`.
fn push(out: &mut BindingLookupScratch, reference: BindingResultEvent, found: &BindingResultEvent, scope: Position, depth: i64, context: ReadContext) {
    out.order.push(reference.ordinal);
    out.rows.push(LookupRow {
        reference: reference.occurrence,
        name: reference.name,
        event: found.occurrence,
        kind: found.kind,
        scope,
        lexical: found.lexical,
        depth: Integer(depth),
        ordinal: found.ordinal,
        context,
    });
}

/// The row of the type parameter of `owner` named as the reference, if any (PEP 695).
fn parameters(owner: Position, reference: BindingResultEvent, depth: i64, context: ReadContext, out: &mut BindingLookupScratch) {
    for at in 0..out.parameters.len() {
        let Some(parameter) = out.parameters.get(at).copied() else {
            break;
        };
        if parameter.owner != owner || parameter.name != reference.name {
            continue;
        }
        out.order.push(reference.ordinal);
        out.rows.push(LookupRow {
            reference: reference.occurrence,
            name: reference.name,
            event: parameter.id,
            kind: EventKind::Bind,
            scope: parameter.id,
            lexical: parameter.id,
            depth: Integer(depth),
            ordinal: Integer(0),
            context,
        });
    }
}
