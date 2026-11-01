//! Schedule language evaluation and alternatives; no linter policy or runtime CFG is encoded.
use crate::domain::{
    definitions::service::definition,
    fields::service::field_of,
    labels::types::Field,
    model::types::{
        BindingAction, BindingAnnotationTiming, BindingPhase, BindingPhaseKind, BindingSchedule,
        BindingStep, Declarations,
    },
    nodes::types::{Category, Node},
    scopes::{
        service::{context, enclosing},
        types::Context,
    },
    semantic_syntax::{service::visibility, types::Visibility},
    syntax::{
        service::{category, is_token, kind, nodes},
        types::{Kind, Leaf},
    },
};
use ryo::languages::{self, Position, Symbols, Tree};

/// Append one explicit event using its source identity.
fn push(plan: &mut BindingSchedule, at: Position, phase: Position, action: BindingAction) {
    plan.steps.push(BindingStep {
        occurrence: at,
        phase,
        action,
    });
}

/// Visit child fields in grammar order; field identity already follows the provider's AST contract.
fn fields(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    phase: Position,
    wanted: &[Field],
    plan: &mut BindingSchedule,
) {
    for field in wanted {
        for child in nodes(tree, at).filter(|child| field_of(tree, *child) == *field) {
            walk(tree, model, child, phase, plan);
        }
    }
}

/// Defaults/annotations execute before entering the callable; parameter bindings do not.
fn parameter_values(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    phase: Position,
    plan: &mut BindingSchedule,
) {
    if category(tree, at) == Category::Expression {
        walk(tree, model, at, phase, plan);
        return;
    }
    for child in nodes(tree, at) {
        parameter_values(tree, model, child, phase, plan);
    }
}

/// Bind parameter identities at body entry without reevaluating their defaults.
fn parameters(tree: &Tree<'_>, at: Position, phase: Position, plan: &mut BindingSchedule) {
    if kind(tree, at) == Kind::Node(Node::Arg) {
        push(plan, at, phase, BindingAction::Bind);
        return;
    }
    if category(tree, at) == Category::Expression {
        return;
    }
    for child in nodes(tree, at) {
        parameters(tree, child, phase, plan);
    }
}

/// Conditional arms are alternatives; a missing else contributes the entry environment.
fn conditional(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    phase: Position,
    plan: &mut BindingSchedule,
) {
    fields(tree, model, at, phase, &[Field::Test], plan);
    push(plan, at, phase, BindingAction::BeginAlternatives);
    fields(tree, model, at, phase, &[Field::Body], plan);
    if kind(tree, at) == Kind::Node(Node::IfExp) {
        push(plan, at, phase, BindingAction::NextAlternative);
        fields(tree, model, at, phase, &[Field::Orelse], plan);
    } else {
        let mut final_arm = None;
        for clause in nodes(tree, at).filter(|child| field_of(tree, *child) == Field::Orelse) {
            push(plan, at, phase, BindingAction::NextAlternative);
            walk(tree, model, clause, phase, plan);
            final_arm = Some(clause);
        }
        if final_arm.is_none_or(|clause| {
            nodes(tree, clause).any(|child| field_of(tree, child) == Field::Test)
        }) {
            push(plan, at, phase, BindingAction::NextAlternative);
        }
    }
    push(plan, at, phase, BindingAction::EndAlternatives);
}

/// The first iterable executes outside a comprehension's own target namespace.
fn comprehension(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    phase: Position,
    deferred: Option<BindingPhaseKind>,
    plan: &mut BindingSchedule,
) {
    let first = nodes(tree, at).find(|child| kind(tree, *child) == Kind::Node(Node::Comprehension));
    if let Some(first) = first {
        fields(tree, model, first, phase, &[Field::Iter], plan);
    }
    if let Some(kind) = deferred {
        plan.phases.push(BindingPhase {
            root: at,
            parent: phase,
            kind,
        });
        return;
    }
    comprehension_body(tree, model, at, phase, first, plan);
}

/// Target/filter evaluation follows each iterable; result expressions follow all generators.
fn comprehension_body(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    phase: Position,
    first: Option<Position>,
    plan: &mut BindingSchedule,
) {
    push(plan, at, phase, BindingAction::BeginAlternatives);
    for clause in
        nodes(tree, at).filter(|child| kind(tree, *child) == Kind::Node(Node::Comprehension))
    {
        if Some(clause) != first {
            fields(tree, model, clause, phase, &[Field::Iter], plan);
        }
        fields(
            tree,
            model,
            clause,
            phase,
            &[Field::Target, Field::Ifs],
            plan,
        );
    }
    fields(
        tree,
        model,
        at,
        phase,
        &[Field::Key, Field::Value, Field::Elt],
        plan,
    );
    // Iterables may be empty and filters may reject every value. External assignment
    // expressions therefore cannot become unconditional stores after the comprehension.
    push(plan, at, phase, BindingAction::NextAlternative);
    push(plan, at, phase, BindingAction::EndAlternatives);
}

/// Walk syntax in language evaluation order. Branch environments are joined by the analysis,
/// not interpreted as one last-visited alternative or a proof of runtime initialization.
#[expect(
    clippy::too_many_lines,
    reason = "the syntax-order schedule must remain one exhaustive AST dispatch over control-flow constructs"
)]
fn walk(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    phase: Position,
    plan: &mut BindingSchedule,
) {
    if is_token(tree, at) == Leaf::Token {
        return;
    }
    if plan.annotations == BindingAnnotationTiming::Deferred
        && at != phase
        && matches!(field_of(tree, at), Field::Annotation | Field::Returns)
        && category(tree, at) == Category::Expression
    {
        plan.phases.push(BindingPhase {
            root: at,
            parent: phase,
            kind: BindingPhaseKind::Type,
        });
        return;
    }
    match kind(tree, at) {
        Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef | Node::Lambda) => {
            for child in nodes(tree, at).filter(|child| field_of(tree, *child) != Field::Body) {
                parameter_values(tree, model, child, phase, plan);
            }
            if matches!(
                kind(tree, at),
                Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef)
            ) {
                push(plan, at, phase, BindingAction::Bind);
            }
            plan.phases.push(BindingPhase {
                root: at,
                parent: phase,
                kind: BindingPhaseKind::Callable,
            });
        }
        Kind::Node(Node::ClassDef) => {
            for child in nodes(tree, at) {
                walk(tree, model, child, phase, plan);
            }
            push(plan, at, phase, BindingAction::Bind);
        }
        Kind::Node(Node::Assign | Node::NamedExpr) => {
            fields(
                tree,
                model,
                at,
                phase,
                &[Field::Value, Field::Targets, Field::Target],
                plan,
            );
        }
        Kind::Node(Node::AnnAssign) => {
            fields(
                tree,
                model,
                at,
                phase,
                &[Field::Value, Field::Annotation],
                plan,
            );
            if nodes(tree, at).any(|child| field_of(tree, child) == Field::Value) {
                fields(tree, model, at, phase, &[Field::Target], plan);
            } else {
                for target in
                    nodes(tree, at).filter(|child| field_of(tree, *child) == Field::Target)
                {
                    if kind(tree, target) == Kind::Node(Node::Name) {
                        push(plan, target, phase, BindingAction::Annotation);
                    } else {
                        // Annotation-only attribute and subscript targets still evaluate their
                        // bases and indices. They do not declare the attribute/subscript itself.
                        walk(tree, model, target, phase, plan);
                    }
                }
            }
        }
        Kind::Node(Node::AugAssign) => {
            for target in nodes(tree, at).filter(|child| field_of(tree, *child) == Field::Target) {
                if kind(tree, target) == Kind::Node(Node::Name) {
                    push(plan, target, phase, BindingAction::Read);
                } else {
                    walk(tree, model, target, phase, plan);
                }
            }
            fields(tree, model, at, phase, &[Field::Value, Field::Target], plan);
        }
        Kind::Node(Node::If | Node::IfExp) => conditional(tree, model, at, phase, plan),
        Kind::Node(Node::BoolOp) => {
            let mut values =
                nodes(tree, at).filter(|child| field_of(tree, *child) == Field::Values);
            if let Some(first) = values.next() {
                walk(tree, model, first, phase, plan);
            }
            for value in values {
                push(plan, at, phase, BindingAction::BeginAlternatives);
                walk(tree, model, value, phase, plan);
                push(plan, at, phase, BindingAction::NextAlternative);
                push(plan, at, phase, BindingAction::EndAlternatives);
            }
        }
        Kind::Node(Node::Match) => {
            fields(tree, model, at, phase, &[Field::Subject], plan);
            push(plan, at, phase, BindingAction::BeginAlternatives);
            for case in nodes(tree, at).filter(|child| field_of(tree, *child) == Field::Cases) {
                walk(tree, model, case, phase, plan);
                push(plan, at, phase, BindingAction::NextAlternative);
            }
            // Conservatively retain the unmatched path; pattern exhaustiveness is not inferred.
            push(plan, at, phase, BindingAction::EndAlternatives);
        }
        Kind::Node(Node::For | Node::AsyncFor | Node::While) => {
            fields(tree, model, at, phase, &[Field::Iter, Field::Test], plan);
            push(plan, at, phase, BindingAction::BeginAlternatives);
            fields(tree, model, at, phase, &[Field::Target, Field::Body], plan);
            push(plan, at, phase, BindingAction::NextAlternative);
            push(plan, at, phase, BindingAction::EndAlternatives);
            fields(tree, model, at, phase, &[Field::Orelse], plan);
        }
        Kind::Node(Node::Try | Node::TryStar) => {
            push(plan, at, phase, BindingAction::BeginAlternatives);
            fields(tree, model, at, phase, &[Field::Body, Field::Orelse], plan);
            for handler in nodes(tree, at).filter(|child| field_of(tree, *child) == Field::Handlers)
            {
                push(plan, at, phase, BindingAction::NextAlternative);
                walk(tree, model, handler, phase, plan);
            }
            push(plan, at, phase, BindingAction::EndAlternatives);
            fields(tree, model, at, phase, &[Field::Finalbody], plan);
        }
        Kind::Node(Node::ExceptHandler) => {
            fields(tree, model, at, phase, &[Field::Type], plan);
            if definition(tree, model, at).is_some() {
                push(plan, at, phase, BindingAction::Bind);
            }
            fields(tree, model, at, phase, &[Field::Body], plan);
            if definition(tree, model, at).is_some() {
                push(plan, at, phase, BindingAction::Cleanup);
            }
        }
        Kind::Node(Node::GeneratorExp) => comprehension(
            tree,
            model,
            at,
            phase,
            Some(BindingPhaseKind::Generator),
            plan,
        ),
        Kind::Node(Node::ListComp | Node::SetComp | Node::DictComp) => {
            comprehension(tree, model, at, phase, None, plan);
        }
        Kind::Node(Node::Name) => {
            push(
                plan,
                at,
                phase,
                match context(tree, at) {
                    Context::Load => BindingAction::Read,
                    Context::Store => BindingAction::Bind,
                    Context::Del => BindingAction::Delete,
                },
            );
        }
        Kind::Node(Node::Global) => push(plan, at, phase, BindingAction::Global),
        Kind::Node(Node::Nonlocal) => push(plan, at, phase, BindingAction::Nonlocal),
        Kind::Node(Node::Arg) => parameter_values(tree, model, at, phase, plan),
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {
            for child in nodes(tree, at) {
                walk(tree, model, child, phase, plan);
            }
            if definition(tree, model, at).is_some() {
                push(plan, at, phase, BindingAction::Bind);
            }
        }
    }
}

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
fn deferred(tree: &Tree<'_>, model: &Declarations, plan: &mut BindingSchedule, next: &mut usize) {
    while *next < plan.phases.len() {
        let Some(task) = plan.phases.get(*next).copied() else {
            break;
        };
        *next += 1;
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
    plan.annotations = if languages::positions(tree.elements)
        .filter(|at| kind(tree, *at) == Kind::Node(Node::Alias))
        .any(|alias| {
            crate::domain::imports::service::import_target(tree, symbols, alias)
                .and_then(|name| languages::text_of(symbols, tree.source, name))
                .is_some_and(|name| name.0 == super::constants::FUTURE_ANNOTATIONS)
        }) {
        BindingAnnotationTiming::Deferred
    } else {
        BindingAnnotationTiming::Immediate
    };
    push(plan, Position(0), Position(0), BindingAction::BeginPhase);
    walk(tree, model, Position(0), Position(0), plan);
    push(plan, Position(0), Position(0), BindingAction::EndPhase);
    let mut next = 0;
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
