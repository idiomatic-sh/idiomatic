//! Schedule language evaluation and alternatives; no linter policy or runtime CFG is encoded.
use super::constants;
use super::types::PhaseCursor;
use crate::domain::{
    binding_walk::service::{comprehension_body, fields, parameters, push, walk},
    labels::types::Field,
    model::types::{
        BindingAction, BindingAnnotationTiming, BindingPhase, BindingPhaseKind, BindingSchedule,
        Declarations,
    },
    nodes::types::Node,
    scopes::service::enclosing,
    semantic_syntax::{service::visibility, types::Visibility},
    syntax::{
        service::{kind, nodes},
        types::Kind,
    },
};
use ryo::languages::{self, Position, Symbols, Tree};

/// Identify the completed phase inherited by a deferred type expression.
fn enclosing_phase(tree: &Tree<'_>, at: Position) -> Position {
    if let Some(embedded) = languages::embedded_source_at(tree.elements, at) {
        return embedded.root;
    }
    let mut scope = enclosing(tree, at);
    loop {
        if scope == Position(0)
            || matches!(
                kind(tree, scope),
                Kind::Node(
                    Node::FunctionDef | Node::AsyncFunctionDef | Node::Lambda | Node::GeneratorExp
                )
            )
        {
            return scope;
        }
        scope = enclosing(tree, scope);
    }
}

/// Drain newly discovered deferred phases only after their parent has completed.
fn deferred(
    tree: &Tree<'_>,
    model: &Declarations,
    plan: &mut BindingSchedule,
    next: &mut PhaseCursor,
) {
    while next.0 < plan.phases.len() {
        let Some(task) = plan.phases.get(next.0).copied() else {
            break;
        };
        next.0 += 1;
        push(plan, task.root, task.root, BindingAction::BeginPhase);
        match task.kind {
            BindingPhaseKind::Callable => {
                for arguments in nodes(tree, task.root)
                    .filter(|child| kind(tree, *child) == Kind::Node(Node::Arguments))
                {
                    parameters(tree, arguments, task.root, plan);
                }
                fields(tree, model, task.root, task.root, &[Field::Body], plan);
            }
            BindingPhaseKind::Generator => {
                let first = nodes(tree, task.root)
                    .find(|child| kind(tree, *child) == Kind::Node(Node::Comprehension));
                comprehension_body(tree, model, task.root, task.root, first, plan);
            }
            BindingPhaseKind::Type => walk(tree, model, task.root, task.root, plan),
        }
        push(plan, task.root, task.root, BindingAction::EndPhase);
    }
}

/// Rebuild a retained schedule on demand. Enabled embeddings are explicit deferred phases.
pub(crate) fn schedule(
    tree: &Tree<'_>,
    model: &Declarations,
    symbols: &mut Symbols,
    plan: &mut BindingSchedule,
) {
    plan.steps.clear();
    plan.phases.clear();
    plan.annotations = if model.language_target >= constants::LAZY_ANNOTATIONS_FROM
        || languages::property_at(
            tree.elements,
            Position(0),
            crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS,
        )
        .is_some_and(|value| {
            value.0 == crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS_ENABLED
        }) {
        BindingAnnotationTiming::Deferred
    } else {
        BindingAnnotationTiming::Immediate
    };
    push(plan, Position(0), Position(0), BindingAction::BeginPhase);
    walk(tree, model, Position(0), Position(0), plan);
    push(plan, Position(0), Position(0), BindingAction::EndPhase);
    let mut next = PhaseCursor(0);
    deferred(tree, model, plan, &mut next);
    // Embeddings are appended parent-first by the frontend, after native syntax is complete.
    for embedding in languages::embedded_expressions(tree.elements) {
        if visibility(tree, model, embedding.root, symbols) != Visibility::Visible {
            continue;
        }
        plan.phases.push(BindingPhase {
            root: embedding.root,
            parent: enclosing_phase(tree, embedding.host),
            kind: BindingPhaseKind::Type,
        });
        deferred(tree, model, plan, &mut next);
    }
}
