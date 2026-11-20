//! The lookup path: for each read or deletion, the namespaces Python looks the name up in, nearest first (Language Reference
//! 4.2.2, 7.12; PEP 695). Only the path is stated: which event of the name is in effect in a namespace is the reader's lookup
//! over binding-event, at the instant and under the rules the reader chooses.
use crate::domain::{
    binding_analysis::service::indexed,
    binding_resolution::service::{context_of, in_header, root_of},
    binding_results::types::{
        BindingResultEvent, EventKind, LookupScopeScratch, PathStep, PathsMade, ReadContext,
        TypeParameter,
    },
    definitions::service::definition,
    model::types::{BindingAnalysisIndex, Declarations},
    nodes::types::Node,
    scopes::service::{declares_global, enclosing},
    syntax::{
        service::{is_root, kind, parent},
        types::{Kind, Rootness},
    },
};
use ryo::languages::{self, Integer, Position, Symbols, Tree};
use std::cell::Ref;

/// The file's lookup paths, made on the first call for the file; `None` when the binding analysis is incomplete.
pub(crate) fn paths<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Option<Ref<'a, LookupScopeScratch>> {
    let index = indexed(tree, model, symbols)?;
    if model.lookup_scope.borrow().made == PathsMade::NotYet {
        let mut out = model.lookup_scope.borrow_mut();
        walk(tree, model, &index, &mut out);
        out.made = PathsMade::Made;
    }
    drop(index);
    Some(model.lookup_scope.borrow())
}

/// Forget the current file's paths while keeping every buffer.
pub(crate) fn clear(out: &mut LookupScopeScratch) {
    out.made = PathsMade::NotYet;
    out.rows.clear();
    out.parameters.clear();
}

/// One pass over the schedule's reads and deletions.
fn walk(
    tree: &Tree<'_>,
    model: &Declarations,
    index: &BindingAnalysisIndex,
    out: &mut LookupScopeScratch,
) {
    clear(out);
    let LookupScopeScratch {
        rows, parameters, ..
    } = out;
    for at in languages::positions(tree.elements) {
        if !matches!(
            kind(tree, at),
            Kind::Node(Node::TypeVar | Node::TypeVarTuple | Node::ParamSpec)
        ) {
            continue;
        }
        if let Some(defined) = definition(tree, model, at) {
            parameters.push(TypeParameter {
                owner: defined.owner,
                id: at,
                name: defined.name,
            });
        }
    }
    let mut cursor = 0;
    for event in &index.events {
        if matches!(event.kind, EventKind::Read | EventKind::Delete) {
            let context = context_of(index, &mut cursor, event.occurrence, event.kind);
            reference(tree, model, parameters, *event, context, rows);
        }
    }
}

/// The path of one reference: the type parameters its header sees, then its own namespace and outward.
fn reference(
    tree: &Tree<'_>,
    model: &Declarations,
    parameters: &[TypeParameter],
    event: BindingResultEvent,
    context: ReadContext,
    out: &mut Vec<PathStep>,
) {
    let start = enclosing(tree, event.occurrence);
    let root = root_of(tree, start);
    let mut depth = Integer(0);
    if !parameters.is_empty() {
        let mut header = false;
        let mut above = event.occurrence;
        while is_root(tree, above) == Rootness::Inner {
            above = parent(tree, above);
            if parameters.iter().any(|each| each.owner == above)
                && in_header(tree, event.occurrence, above, start)
            {
                header = true;
                named(parameters, above, event, depth, context, out);
            }
        }
        if header {
            depth = Integer(depth.0 + 1);
        }
    }
    let mut scope = start;
    let mut first = true;
    loop {
        // 4.2.2: a class block's names are not visible from the blocks nested in it.
        let skipped = !first && kind(tree, scope) == Kind::Node(Node::ClassDef);
        if !skipped {
            step(out, event, scope, depth, context);
            depth = Integer(depth.0 + 1);
            // 7.12: after a block that declares the name global, the name is the module's.
            if scope != root && declares_global(tree, model, scope, event.name) {
                step(out, event, root, depth, context);
                break;
            }
        }
        if scope == root {
            break;
        }
        if parameters.iter().any(|each| each.owner == scope) {
            named(parameters, scope, event, depth, context, out);
            depth = Integer(depth.0 + 1);
        }
        scope = enclosing(tree, scope);
        first = false;
    }
}

/// One row: the namespace `scope` on the path of `reference`.
fn step(
    out: &mut Vec<PathStep>,
    reference: BindingResultEvent,
    scope: Position,
    depth: Integer,
    context: ReadContext,
) {
    out.push(PathStep {
        reference: reference.occurrence,
        name: reference.name,
        scope,
        depth,
        context,
    });
}

/// The scope of each type parameter of `owner` named as the reference, if any (PEP 695).
fn named(
    parameters: &[TypeParameter],
    owner: Position,
    reference: BindingResultEvent,
    depth: Integer,
    context: ReadContext,
    out: &mut Vec<PathStep>,
) {
    for parameter in parameters {
        if parameter.owner == owner && parameter.name == reference.name {
            step(out, reference, parameter.id, depth, context);
        }
    }
}
