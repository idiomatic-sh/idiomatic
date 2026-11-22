//! The `schedule` service of the model: free functions named by verb, with no IO and no ports. One walk of the parser's nodes in
//! the language's evaluation order writes the steps.

use super::types::ScheduleSource;
use crate::domain::model::types::{
    BindingAction, BindingAnnotationTiming, BindingPhase, BindingPhaseKind, BindingSchedule, BindingStep,
};
use crate::domain::model::definitions::service::definition_of;
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::model::lexical::service::{entry, id_of, node_at, scope_at, subtree_end_of, syntax_form};
use crate::domain::model::lexical::types::{AnnotationMark, NameUse, NodeClass, SyntaxForm};
use ruff_python_ast::{AnyNodeRef, Comprehension, Expr, Stmt};
use ryo::languages::Position;

/// Spike (6f probe): records the phase the position `at` is evaluated in; a later visit (a deferred phase's own walk)
/// overwrites the first.
fn record(model: &mut BindingSchedule, at: Position, phase: Position) {
    let place = at.0 as usize;
    if model.region.len() <= place {
        model.region.resize(place + 1, Position(u32::MAX));
    }
    model.region[place] = phase;
}

/// Writes one step.
fn push(model: &mut BindingSchedule, at: Position, phase: Position, action: BindingAction) {
    model.steps.push(BindingStep { occurrence: at, phase, action });
}

/// The first child of the node `at`; the node's subtree end when it has none.
const fn first_child(at: Position) -> Position {
    Position(at.0.saturating_add(1))
}

/// Walks one expression.
fn walk_expression(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, expression: &Expr, phase: Position) {
    walk(model, source, id_of(expression), phase);
}

/// Walks the statements of a body, in order.
fn walk_body(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, body: &[Stmt], phase: Position) {
    for statement in body {
        walk(model, source, id_of(statement), phase);
    }
}

/// Walks every child of the node `at`, in source order.
fn walk_children(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, at: Position, phase: Position) {
    let end = subtree_end_of(source.tree, at);
    let mut child = first_child(at);
    while child < end {
        walk(model, source, child, phase);
        child = subtree_end_of(source.tree, child);
    }
}

/// Walks the expressions under `at` that run before a callable is entered: its defaults and annotations.
fn parameter_values(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, at: Position, phase: Position) {
    record(model, at, phase);
    if syntax_form(entry(&source.lexical.class, at)) == SyntaxForm::Expression {
        walk(model, source, at, phase);
        return;
    }
    let end = subtree_end_of(source.tree, at);
    let mut child = first_child(at);
    while child < end {
        parameter_values(model, source, child, phase);
        child = subtree_end_of(source.tree, child);
    }
}

/// Binds the parameters under `at`: they are bound when the body is entered.
fn parameters(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, at: Position, phase: Position) {
    let class = entry(&source.lexical.class, at);
    if class == NodeClass::Parameter {
        record(model, at, phase);
        push(model, at, phase, BindingAction::Bind);
        return;
    }
    if syntax_form(class) == SyntaxForm::Expression {
        return;
    }
    let end = subtree_end_of(source.tree, at);
    let mut child = first_child(at);
    while child < end {
        parameters(model, source, child, phase);
        child = subtree_end_of(source.tree, child);
    }
}

/// Walks a comprehension: its first iterable where it stands, then its body, now or (a generator's) in a later phase.
fn comprehension(
    model: &mut BindingSchedule,
    source: ScheduleSource<'_, '_, '_>,
    at: Position,
    generators: &[Comprehension],
    phase: Position,
    kind: Option<BindingPhaseKind>,
) {
    if let Some(first) = generators.first() {
        walk_expression(model, source, &first.iter, phase);
    }
    if let Some(kind) = kind {
        model.phases.push(BindingPhase { root: at, parent: phase, kind });
        return;
    }
    comprehension_body(model, source, at, phase);
}

/// Walks the body of the comprehension `at`: its clauses after the first iterable, then its results; it may run no time at all.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "four of the parser's node kinds are comprehensions; no other node reaches this function"
)]
fn comprehension_body(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, at: Position, phase: Position) {
    let Some(node) = node_at(source.tree, at) else {
        return;
    };
    push(model, at, phase, BindingAction::BeginAlternatives);
    let (generators, results): (&[Comprehension], [Option<&Expr>; 2]) = match node {
        AnyNodeRef::ExprListComp(typed) => (&typed.generators, [Some(&*typed.elt), None]),
        AnyNodeRef::ExprSetComp(typed) => (&typed.generators, [Some(&*typed.elt), None]),
        AnyNodeRef::ExprGenerator(typed) => (&typed.generators, [Some(&*typed.elt), None]),
        AnyNodeRef::ExprDictComp(typed) => (&typed.generators, [typed.key.as_deref(), Some(&*typed.value)]),
        _ => (&[], [None, None]),
    };
    for (number, clause) in generators.iter().enumerate() {
        if number != 0 {
            walk_expression(model, source, &clause.iter, phase);
        }
        walk_expression(model, source, &clause.target, phase);
        for condition in &clause.ifs {
            walk_expression(model, source, condition, phase);
        }
    }
    for result in results.into_iter().flatten() {
        walk_expression(model, source, result, phase);
    }
    push(model, at, phase, BindingAction::NextAlternative);
    push(model, at, phase, BindingAction::EndAlternatives);
}

/// Walks a function or a lambda where it stands: everything but its body (decorators, type parameters, defaults, annotations),
/// then the function's own binding; its body is a later phase.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "two of the parser's node kinds are callables; no other node reaches this function"
)]
fn callable(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, at: Position, phase: Position) {
    let Some(node) = node_at(source.tree, at) else {
        return;
    };
    let body = match node {
        AnyNodeRef::StmtFunctionDef(function) => function.body.first().map(id_of),
        AnyNodeRef::ExprLambda(lambda) => Some(id_of(&*lambda.body)),
        _ => None,
    };
    let end = subtree_end_of(source.tree, at);
    let mut child = first_child(at);
    while child < end && Some(child) != body {
        parameter_values(model, source, child, phase);
        child = subtree_end_of(source.tree, child);
    }
    if entry(&source.lexical.class, at) == NodeClass::FunctionDef {
        push(model, at, phase, BindingAction::Bind);
    }
    model.phases.push(BindingPhase { root: at, parent: phase, kind: BindingPhaseKind::Callable });
}

/// Walks the node `at` in the language's evaluation order.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "twenty-two of the parser's node kinds are evaluated in an order of their own; every other in source order"
)]
#[expect(clippy::too_many_lines, reason = "one arm per construct with an evaluation order of its own")]
fn walk(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, at: Position, phase: Position) {
    let Some(node) = node_at(source.tree, at) else {
        return;
    };
    record(model, at, phase);
    let mark = entry(&source.lexical.annotation, at);
    if at != phase
        && (mark == AnnotationMark::Unevaluated
            || (source.timing == BindingAnnotationTiming::Deferred && mark == AnnotationMark::Annotation))
    {
        model.phases.push(BindingPhase { root: at, parent: phase, kind: BindingPhaseKind::Type });
        return;
    }
    match node {
        AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::ExprLambda(_) => callable(model, source, at, phase),
        AnyNodeRef::StmtClassDef(_) => {
            walk_children(model, source, at, phase);
            push(model, at, phase, BindingAction::Bind);
        }
        AnyNodeRef::StmtAssign(statement) => {
            walk_expression(model, source, &statement.value, phase);
            for target in &statement.targets {
                walk_expression(model, source, target, phase);
            }
        }
        AnyNodeRef::ExprNamed(expression) => {
            walk_expression(model, source, &expression.value, phase);
            walk_expression(model, source, &expression.target, phase);
        }
        AnyNodeRef::StmtAnnAssign(statement) => {
            if let Some(value) = &statement.value {
                walk_expression(model, source, value, phase);
            }
            walk_expression(model, source, &statement.annotation, phase);
            if statement.value.is_none() && statement.target.is_name_expr() {
                push(model, id_of(&*statement.target), phase, BindingAction::Annotation);
            } else {
                walk_expression(model, source, &statement.target, phase);
            }
        }
        AnyNodeRef::StmtAugAssign(statement) => {
            if statement.target.is_name_expr() {
                push(model, id_of(&*statement.target), phase, BindingAction::Read);
            } else {
                walk_expression(model, source, &statement.target, phase);
            }
            walk_expression(model, source, &statement.value, phase);
            walk_expression(model, source, &statement.target, phase);
        }
        AnyNodeRef::StmtIf(statement) => {
            walk_expression(model, source, &statement.test, phase);
            push(model, at, phase, BindingAction::BeginAlternatives);
            walk_body(model, source, &statement.body, phase);
            for clause in &statement.elif_else_clauses {
                push(model, at, phase, BindingAction::NextAlternative);
                walk(model, source, id_of(clause), phase);
            }
            if statement.elif_else_clauses.last().is_none_or(|clause| clause.test.is_some()) {
                push(model, at, phase, BindingAction::NextAlternative);
            }
            push(model, at, phase, BindingAction::EndAlternatives);
        }
        AnyNodeRef::ExprIf(expression) => {
            walk_expression(model, source, &expression.test, phase);
            push(model, at, phase, BindingAction::BeginAlternatives);
            walk_expression(model, source, &expression.body, phase);
            push(model, at, phase, BindingAction::NextAlternative);
            walk_expression(model, source, &expression.orelse, phase);
            push(model, at, phase, BindingAction::EndAlternatives);
        }
        AnyNodeRef::ExprBoolOp(expression) => {
            let mut values = expression.values.iter();
            if let Some(first) = values.next() {
                walk_expression(model, source, first, phase);
            }
            for value in values {
                push(model, at, phase, BindingAction::BeginAlternatives);
                walk_expression(model, source, value, phase);
                push(model, at, phase, BindingAction::NextAlternative);
                push(model, at, phase, BindingAction::EndAlternatives);
            }
        }
        AnyNodeRef::StmtMatch(statement) => {
            walk_expression(model, source, &statement.subject, phase);
            push(model, at, phase, BindingAction::BeginAlternatives);
            for case in &statement.cases {
                walk(model, source, id_of(case), phase);
                push(model, at, phase, BindingAction::NextAlternative);
            }
            push(model, at, phase, BindingAction::EndAlternatives);
        }
        AnyNodeRef::StmtFor(statement) => {
            walk_expression(model, source, &statement.iter, phase);
            push(model, at, phase, BindingAction::BeginAlternatives);
            walk_expression(model, source, &statement.target, phase);
            walk_body(model, source, &statement.body, phase);
            push(model, at, phase, BindingAction::NextAlternative);
            push(model, at, phase, BindingAction::EndAlternatives);
            walk_body(model, source, &statement.orelse, phase);
        }
        AnyNodeRef::StmtWhile(statement) => {
            walk_expression(model, source, &statement.test, phase);
            push(model, at, phase, BindingAction::BeginAlternatives);
            walk_body(model, source, &statement.body, phase);
            push(model, at, phase, BindingAction::NextAlternative);
            push(model, at, phase, BindingAction::EndAlternatives);
            walk_body(model, source, &statement.orelse, phase);
        }
        AnyNodeRef::StmtTry(statement) => {
            push(model, at, phase, BindingAction::BeginAlternatives);
            walk_body(model, source, &statement.body, phase);
            walk_body(model, source, &statement.orelse, phase);
            for handler in &statement.handlers {
                push(model, at, phase, BindingAction::NextAlternative);
                walk(model, source, id_of(handler), phase);
            }
            push(model, at, phase, BindingAction::EndAlternatives);
            walk_body(model, source, &statement.finalbody, phase);
        }
        AnyNodeRef::ExceptHandlerExceptHandler(handler) => {
            if let Some(caught) = &handler.type_ {
                walk_expression(model, source, caught, phase);
            }
            let named = definition_of(source.definitions, at).is_some();
            if named {
                push(model, at, phase, BindingAction::Bind);
            }
            walk_body(model, source, &handler.body, phase);
            if named {
                push(model, at, phase, BindingAction::Cleanup);
            }
        }
        AnyNodeRef::ExprGenerator(typed) => {
            comprehension(model, source, at, &typed.generators, phase, Some(BindingPhaseKind::Generator));
        }
        AnyNodeRef::ExprListComp(typed) => comprehension(model, source, at, &typed.generators, phase, None),
        AnyNodeRef::ExprSetComp(typed) => comprehension(model, source, at, &typed.generators, phase, None),
        AnyNodeRef::ExprDictComp(typed) => comprehension(model, source, at, &typed.generators, phase, None),
        AnyNodeRef::ExprName(_) => {
            let action = match entry(&source.lexical.usage, at) {
                NameUse::Load => BindingAction::Read,
                NameUse::Store => BindingAction::Bind,
                NameUse::Delete => BindingAction::Delete,
            };
            push(model, at, phase, action);
        }
        AnyNodeRef::StmtGlobal(_) => push(model, at, phase, BindingAction::Global),
        AnyNodeRef::StmtNonlocal(_) => push(model, at, phase, BindingAction::Nonlocal),
        AnyNodeRef::Parameter(_) => parameter_values(model, source, at, phase),
        _ => {
            walk_children(model, source, at, phase);
            if definition_of(source.definitions, at).is_some() {
                push(model, at, phase, BindingAction::Bind);
            }
        }
    }
}

/// Walks the body of the later phase `task`.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "two of the parser's node kinds are callables; a generator's body is walked as a comprehension's"
)]
fn later(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, task: BindingPhase) {
    match task.kind {
        BindingPhaseKind::Generator => return comprehension_body(model, source, task.root, task.root),
        BindingPhaseKind::Type => return walk(model, source, task.root, task.root),
        BindingPhaseKind::Callable => {}
    }
    let Some(node) = node_at(source.tree, task.root) else {
        return;
    };
    match node {
        AnyNodeRef::StmtFunctionDef(function) => {
            parameters(model, source, id_of(&*function.parameters), task.root);
            walk_body(model, source, &function.body, task.root);
        }
        AnyNodeRef::ExprLambda(lambda) => {
            if let Some(typed) = &lambda.parameters {
                parameters(model, source, id_of(&**typed), task.root);
            }
            walk_expression(model, source, &lambda.body, task.root);
        }
        _ => {}
    }
}

/// Builds the schedule of one file: the module's phase, then every later phase in the order it was defined.
pub(crate) fn build(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>) {
    model.steps.clear();
    model.phases.clear();
    model.region.clear();
    model.annotations = source.timing;
    push(model, MODULE, MODULE, BindingAction::BeginPhase);
    walk(model, source, MODULE, MODULE);
    push(model, MODULE, MODULE, BindingAction::EndPhase);
    let mut next = 0;
    drain(model, source, &mut next);
    // The expression of a string the language reads as a type runs after the file's own schedule, a phase of its own that
    // inherits the finished phase around its string; the segments stand parent first.
    for segment in source.tree.segments {
        model.phases.push(BindingPhase {
            root: segment.first,
            parent: enclosing_phase(source, segment.host),
            kind: BindingPhaseKind::Type,
        });
        drain(model, source, &mut next);
    }
}

/// Writes every later phase not yet written, in the order they were defined: a phase may define more.
fn drain(model: &mut BindingSchedule, source: ScheduleSource<'_, '_, '_>, next: &mut usize) {
    while let Some(task) = model.phases.get(*next).copied() {
        *next = usize::saturating_add(*next, 1);
        push(model, task.root, task.root, BindingAction::BeginPhase);
        later(model, source, task);
        push(model, task.root, task.root, BindingAction::EndPhase);
    }
}

/// The finished phase the expression of the string at `host` inherits: the embedded segment that holds the string, or the
/// nearest scope around it that is a phase (the module, a callable, a generator).
fn enclosing_phase(source: ScheduleSource<'_, '_, '_>, host: Position) -> Position {
    let held = source
        .tree
        .segments
        .iter()
        .find(|segment| segment.first <= host && host < subtree_end_of(source.tree, segment.first));
    if let Some(segment) = held {
        return segment.first;
    }
    let mut scope = scope_at(source.lexical, host);
    loop {
        if scope == MODULE
            || matches!(
                entry(&source.lexical.class, scope),
                NodeClass::FunctionDef | NodeClass::Lambda | NodeClass::Generator
            )
        {
            return scope;
        }
        scope = scope_at(source.lexical, scope);
    }
}
