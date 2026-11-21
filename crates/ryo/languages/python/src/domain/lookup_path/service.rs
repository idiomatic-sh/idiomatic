//! The lookup path: for each read or deletion, the namespaces Python looks the name up in, nearest first (Language Reference
//! 4.2.2, 7.12; PEP 695). Only the path is stated: which event of the name is in effect in a namespace is the reader's lookup
//! over binding-event, at the instant and under the rules the reader chooses.
//!
//! The path is walked over each of the two readings of a file: `paths` over today's parse, for the readers, and `stepped`
//! over the semantic model, for the walk's fillers. The walk's contract holds the two to the same rows.
use crate::domain::{
    binding_analysis::service::indexed,
    binding_resolution::service::{context_of, in_header, root_of},
    binding_results::types::{
        BindingResultEvent, EventKind, LookupScopeScratch, PathStep, PathsMade, ReadContext,
        TypeParameter,
    },
    definitions::service::definition,
    model::declarations::service::redirect_of,
    model::definitions::service::definition_of,
    model::flow::types::FlowRead,
    model::lexical::constants::MODULE,
    model::lexical::service::{entry, id_of, node_at, parent_of, scope_at},
    model::lexical::types::{NodeClass, ScopeRedirect},
    model::types::{BindingAnalysisIndex, Declarations},
    model::view::types::ModelRead,
    nodes::types::Node,
    scopes::service::{declares_global, enclosing},
    syntax::{
        service::{is_root, kind, parent},
        types::{Kind, Rootness},
    },
};
use ruff_python_ast::{AnyNodeRef, TypeParam};
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

/// The type parameters the definition `owner` declares: a function's, a class's or a type alias's; none for any other node. A
/// definition that declares one has an annotation scope between itself and its own namespace.
fn type_parameters<'a>(read: ModelRead<'a>, owner: Position) -> &'a [TypeParam] {
    if !matches!(
        entry(&read.model.lexical.class, owner),
        NodeClass::FunctionDef | NodeClass::ClassDef | NodeClass::TypeAlias
    ) {
        return &[];
    }
    let Some(node) = node_at(read.tree, owner) else {
        return &[];
    };
    let declared = if let AnyNodeRef::StmtFunctionDef(function) = node {
        function.type_params.as_deref()
    } else if let AnyNodeRef::StmtClassDef(class) = node {
        class.type_params.as_deref()
    } else if let AnyNodeRef::StmtTypeAlias(alias) = node {
        alias.type_params.as_deref()
    } else {
        None
    };
    declared.map_or(&[], |parameters| &parameters.type_params)
}

/// Tells `step` the scope of each of the type parameters `parameters` named as the read, at `depth` (PEP 695).
fn named_in(
    read: ModelRead<'_>,
    parameters: &[TypeParam],
    each: &FlowRead,
    depth: Integer,
    step: &mut dyn FnMut(Position, Integer),
) {
    for parameter in parameters {
        let id = id_of(parameter);
        if definition_of(&read.model.definitions, id).is_some_and(|found| found.name == each.name) {
            step(id, depth);
        }
    }
}

/// Tells `step` the type parameters the read sees from a header: of each definition between the read and the scope it is
/// evaluated in whose bases, annotations, return, type parameters or alias value hold the read, not its decorators and not its
/// own parameters' defaults. Answers the depth of the read's own scope: 1 after such a definition's type parameters, which are
/// a namespace before it whether or not one of them has the name, and 0 with none.
fn headed(
    read: ModelRead<'_>,
    each: &FlowRead,
    step: &mut dyn FnMut(Position, Integer),
) -> Integer {
    let (lexical, tree) = (&read.model.lexical, read.tree);
    let mut own = Integer(0);
    // The callable whose parameter's default holds the read: a default's parameter, its parameters, then the callable.
    let mut defaulted = None;
    let mut child = each.at;
    let mut above = parent_of(tree, child);
    while above != each.scope && above != child {
        if entry(&lexical.class, above) == NodeClass::ParameterWithDefault
            && entry(&lexical.class, child) != NodeClass::Parameter
        {
            defaulted = Some(parent_of(tree, parent_of(tree, above)));
        }
        let parameters = type_parameters(read, above);
        if !parameters.is_empty()
            && defaulted != Some(above)
            && !matches!(node_at(tree, child), Some(AnyNodeRef::Decorator(_)))
        {
            own = Integer(1);
            named_in(read, parameters, each, Integer(0), step);
        }
        child = above;
        above = parent_of(tree, above);
    }
    own
}

/// Tells `step` each namespace the name of the read `each` is looked up in, nearest first, with its depth, over the semantic
/// model: the type parameters a header sees, then the scope the read is evaluated in and each scope around it but the class
/// blocks, the module directly after a scope that declares the name global, and the type parameters of a scope between it and
/// the scope around it. Namespaces that share a step share a depth; a step with type parameters none of which has the name
/// keeps its depth and tells nothing.
pub(crate) fn stepped(
    read: ModelRead<'_>,
    each: &FlowRead,
    step: &mut dyn FnMut(Position, Integer),
) {
    let lexical = &read.model.lexical;
    let mut depth = headed(read, each, step);
    let mut scope = each.scope;
    loop {
        // 4.2.2: a class block's names are not visible from the blocks nested in it.
        let skipped = scope != each.scope && entry(&lexical.class, scope) == NodeClass::ClassDef;
        if !skipped {
            step(scope, depth);
            depth = Integer(depth.0.saturating_add(1));
            // 7.12: after a block that declares the name global, the name is the module's.
            if scope != MODULE
                && redirect_of(lexical, scope, each.name) == Some(ScopeRedirect::Global)
            {
                step(MODULE, depth);
                return;
            }
        }
        if scope == MODULE {
            return;
        }
        let parameters = type_parameters(read, scope);
        if !parameters.is_empty() {
            named_in(read, parameters, each, depth, step);
            depth = Integer(depth.0.saturating_add(1));
        }
        scope = scope_at(lexical, scope);
    }
}
