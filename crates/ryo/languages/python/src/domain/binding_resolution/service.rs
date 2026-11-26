//! Schedule-order resolution (Spike ryo-fmx3): run the file's schedule in order, keep the latest event of each (namespace, name),
//! and at each read or deletion walk Python's lookup path (Language Reference §4.2.2, §7.12, §7.13; PEP 695), one row per namespace
//! that holds an event of the name. Alternatives are linearised in schedule order; no linter policy is applied here.
use super::types::{BindingResolutionScratch, Depth, Resolution};
use crate::domain::{
    binding_analysis::service::indexed,
    binding_results::types::{EventKind, ReadContext, TypeParameter},
    definitions::service::definition,
    fields::service::field_of,
    labels::types::Field,
    model::types::{BindingAnalysisIndex, BindingAnalysisState, BindingKey, Declarations},
    nodes::types::Node,
    scopes::service::{declared, declares_global, enclosing},
    syntax::{
        service::{is_root, kind, parent},
        types::{Kind, Rootness},
    },
};
use ryo::languages::{self, Integer, Position, Symbol, Symbols, Tree};
use std::cell::Ref;

/// The file's resolution, made on the first call for the file; `None` when the binding analysis is incomplete.
pub(crate) fn resolved<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Option<Ref<'a, BindingResolutionScratch>> {
    let index = indexed(tree, model, symbols)?;
    if model.binding_resolution.borrow().state == BindingAnalysisState::Fresh {
        let mut out = model.binding_resolution.borrow_mut();
        resolve(tree, model, &index, &mut out);
        out.state = BindingAnalysisState::Complete;
    }
    drop(index);
    Some(model.binding_resolution.borrow())
}

/// Forget the current file's resolution while keeping every buffer.
pub(crate) fn clear(out: &mut BindingResolutionScratch) {
    out.state = BindingAnalysisState::Fresh;
    out.resolutions.clear();
    out.latest.clear();
    out.parameters.clear();
    out.lazy.clear();
    out.order.clear();
    out.sorting.clear();
}

/// What one reference's walk reads: the tree, the model, the events and the state of the schedule so far.
struct Walk<'t, 'a> {
    tree: &'t Tree<'t>,
    model: &'a Declarations,
    index: &'a BindingAnalysisIndex,
}

/// One pass over the schedule's events; lazy reads are resolved against the state at the end of the schedule.
fn resolve(
    tree: &Tree<'_>,
    model: &Declarations,
    index: &BindingAnalysisIndex,
    out: &mut BindingResolutionScratch,
) {
    clear(out);
    parameters(tree, model, out);
    let walk = Walk { tree, model, index };
    // C3 (§7.12, §7.13): a global or nonlocal statement is a declaration that holds for the entire code block, so a declaring
    // namespace holds its declaration for every reference, even one the schedule reaches before the statement (which Python then
    // refuses to compile). The row keeps the declaration's own ordinal. A module-level declaration routes nothing and is left out.
    for (at, event) in index.events.iter().enumerate() {
        if matches!(event.kind, EventKind::Global | EventKind::Nonlocal) && event.lexical != event.scope {
            let _first = out.latest.entry(key(event.lexical, event.name)).or_insert(at);
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
                    let _previous = out.latest.insert(key(event.scope, event.name), at);
                }
            }
            EventKind::Bind | EventKind::Annotation | EventKind::Cleanup => {
                let _previous = out.latest.insert(key(event.scope, event.name), at);
            }
            EventKind::Global | EventKind::Nonlocal => {
                let _previous = out.latest.insert(key(event.scope, event.name), at);
                if event.lexical != event.scope {
                    let _declaring = out.latest.insert(key(event.lexical, event.name), at);
                }
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
        rows.extend(out.order.iter().copied().zip(out.resolutions.iter().copied()));
        rows.sort_by_key(|(ordinal, row)| (ordinal.0, row.depth.0.0));
        out.resolutions.clear();
        out.order.clear();
        for (ordinal, row) in &rows {
            out.order.push(*ordinal);
            out.resolutions.push(*row);
        }
        out.sorting = rows;
    }
}

/// The value, type or deletion context the binding analysis recorded for the reference.
pub(crate) fn context_of(
    index: &BindingAnalysisIndex,
    cursor: &mut usize,
    reference: Position,
    event: EventKind,
) -> ReadContext {
    while let Some(read) = index.reads.get(*cursor) {
        *cursor += 1;
        if read.reference == reference {
            return read.context;
        }
    }
    if event == EventKind::Delete {
        ReadContext::Deletion
    } else {
        ReadContext::Value
    }
}

/// The key of a name in a namespace.
pub(crate) const fn key(scope: Position, name: Symbol) -> BindingKey {
    BindingKey { scope, name }
}

/// The file's type parameters, each with its owner (PEP 695: an annotation scope per owner).
fn parameters(tree: &Tree<'_>, model: &Declarations, out: &mut BindingResolutionScratch) {
    for at in languages::positions(tree.elements) {
        if !matches!(
            kind(tree, at),
            Kind::Node(Node::TypeVar | Node::TypeVarTuple | Node::ParamSpec)
        ) {
            continue;
        }
        if let Some(defined) = definition(tree, model, at) {
            out.parameters.push(TypeParameter {
                owner: defined.owner,
                id: at,
                name: defined.name,
            });
        }
    }
}

/// Whether the read at `at` is in a type parameter's bound or default: evaluated lazily, after the whole schedule (PEP 695).
pub(crate) fn lazy(tree: &Tree<'_>, mut at: Position) -> bool {
    loop {
        if is_root(tree, at) == Rootness::Root {
            return false;
        }
        if matches!(field_of(tree, at), Field::Bound | Field::DefaultValue) {
            return true;
        }
        at = parent(tree, at);
    }
}

/// Whether `inner` is `outer` or inside it.
fn within(tree: &Tree<'_>, mut inner: Position, outer: Position) -> bool {
    loop {
        if inner == outer {
            return true;
        }
        if is_root(tree, inner) == Rootness::Root {
            return false;
        }
        inner = parent(tree, inner);
    }
}

/// Whether the reference at `at` is in the header of the owner `owner` (its bases, annotations, return, type parameters or alias
/// value) where the owner's type parameters are visible: not in its body, and not under its decorators or its defaults (§8.7, §8.8).
pub(crate) fn in_header(tree: &Tree<'_>, at: Position, owner: Position, start: Position) -> bool {
    if within(tree, start, owner) {
        return false;
    }
    let mut child = at;
    while child != owner && is_root(tree, child) == Rootness::Inner {
        let field = field_of(tree, child);
        let above = parent(tree, child);
        if above == owner && field == Field::DecoratorList {
            return false;
        }
        if matches!(field, Field::Defaults | Field::KwDefaults) && callable_of(tree, child) == owner {
            return false;
        }
        child = above;
    }
    true
}

/// The nearest function or lambda above `at`.
fn callable_of(tree: &Tree<'_>, mut at: Position) -> Position {
    loop {
        if is_root(tree, at) == Rootness::Root {
            return at;
        }
        at = parent(tree, at);
        if matches!(
            kind(tree, at),
            Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef | Node::Lambda)
        ) {
            return at;
        }
    }
}

impl Walk<'_, '_> {
    /// Emit the rows of the reference at event index `at`: header type parameters, then the path from its own namespace.
    fn reference(&self, at: usize, context: ReadContext, out: &mut BindingResolutionScratch) {
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
                if out.parameters.iter().any(|each| each.owner == above)
                    && in_header(tree, event.occurrence, above, start)
                {
                    header = true;
                    self.parameters(above, event, depth, context, out);
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
            // 4.2.2, 7.5: a deletion binds; a name a block deletes and declares neither global nor nonlocal is the block's own.
            if context == ReadContext::Deletion && declared(tree, self.model, scope, event.name).is_none() {
                break;
            }
            if scope == root {
                break;
            }
            if out.parameters.iter().any(|each| each.owner == scope) {
                self.parameters(scope, event, depth, context, out);
                depth += 1;
            }
            scope = enclosing(tree, scope);
            first = false;
        }
    }

    /// The row of the namespace `scope`, when it holds an event of the reference's name.
    fn namespace(
        &self,
        scope: Position,
        reference: crate::domain::binding_results::types::BindingResultEvent,
        depth: i64,
        context: ReadContext,
        out: &mut BindingResolutionScratch,
    ) {
        let Some(found) = out
            .latest
            .get(&key(scope, reference.name))
            .and_then(|at| self.index.events.get(*at))
        else {
            return;
        };
        out.order.push(reference.ordinal);
        out.resolutions.push(Resolution {
            reference: reference.occurrence,
            event: found.occurrence,
            kind: found.kind,
            scope,
            lexical: found.lexical,
            depth: Depth(Integer(depth)),
            ordinal: found.ordinal,
            context,
        });
    }

    /// The row of the type parameter of `owner` named as the reference, if any (C5).
    fn parameters(
        &self,
        owner: Position,
        reference: crate::domain::binding_results::types::BindingResultEvent,
        depth: i64,
        context: ReadContext,
        out: &mut BindingResolutionScratch,
    ) {
        for at in 0..out.parameters.len() {
            let Some(parameter) = out.parameters.get(at).copied() else {
                break;
            };
            if parameter.owner != owner || parameter.name != reference.name {
                continue;
            }
            let id = parameter.id;
            out.order.push(reference.ordinal);
            out.resolutions.push(Resolution {
                reference: reference.occurrence,
                event: id,
                kind: EventKind::Bind,
                scope: id,
                lexical: id,
                depth: Depth(Integer(depth)),
                ordinal: Integer(0),
                context,
            });
        }
    }
}

/// The module of the file.
pub(crate) fn root_of(tree: &Tree<'_>, mut at: Position) -> Position {
    while is_root(tree, at) == Rootness::Inner {
        at = parent(tree, at);
    }
    at
}
