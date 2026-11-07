//! Lexical name resolution, independent of any lint policy. Resolution associates a read with a
//! declaration; it does not assert that control flow has initialized that declaration.

use ryo::languages::{self, Position, Symbol, Tree};

use super::types::{BindingAvailability, EvaluationReach, ScopeExecution};
use crate::domain::definitions::service::definition;
use crate::domain::model::types::{BindingKey, Declarations, LexicalBinding, QualificationState};
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::{enclosing, redirected};
use crate::domain::syntax::service::{kind, last, parent};
use crate::domain::syntax::types::Kind;

/// Builds the declarations and the binding index now, as a reader's first lookup would: what the tests of the index read.
#[cfg(test)]
pub(crate) fn index(tree: &Tree<'_>, model: &mut Declarations) {
    model.bindings_state.set(QualificationState::Fresh);
    bound(tree, model);
}

/// Index only newly appended definitions; native declarations and qualification remain unchanged. Indexes not built yet are
/// left for the first lookup, which builds them over every element then present.
pub(crate) fn extend(tree: &Tree<'_>, from: Position, model: &mut Declarations) {
    if model.bindings_state.get() == QualificationState::Fresh {
        return;
    }
    let mut bindings = std::mem::take(model.bindings.get_mut());
    collect(tree, from, model, &mut bindings);
    *model.bindings.get_mut() = bindings;
}

/// Builds the binding index of the file on its first lookup; later lookups read it as built.
pub(crate) fn bound(tree: &Tree<'_>, model: &Declarations) {
    if model.bindings_state.get() == QualificationState::Ready {
        return;
    }
    model.bindings_state.set(QualificationState::Ready);
    let mut bindings = model.bindings.take();
    bindings.clear();
    collect(tree, Position(0), model, &mut bindings);
    drop(model.bindings.replace(bindings));
}

/// Pushes every definition from `from` to the end of the table, keeping the index sorted.
fn collect(tree: &Tree<'_>, from: Position, model: &Declarations, bindings: &mut Vec<LexicalBinding>) {
    let previous = bindings.len();
    for at in (from.0..languages::end_of(tree.elements).0).map(Position) {
        if let Some(found) = definition(tree, model, at) {
            bindings.push(LexicalBinding {
                key: BindingKey {
                    scope: found.owner,
                    name: found.name,
                },
                position: at,
            });
        }
    }
    if bindings.len() != previous {
        bindings.sort_unstable();
    }
}

/// A parsed string's bindings apply within its own expression and nested parsed
/// expressions, never to native code or an unrelated quoted expression.
fn evaluation_reach(tree: &Tree<'_>, binding: Position, occurrence: Position) -> EvaluationReach {
    let Some(binding_source) = languages::embedded_source_at(tree.elements, binding) else {
        return EvaluationReach::Shared;
    };
    let mut occurrence = occurrence;
    while let Some(source) = languages::embedded_source_at(tree.elements, occurrence) {
        if source.root == binding_source.root {
            return EvaluationReach::Shared;
        }
        occurrence = source.host;
    }
    EvaluationReach::Isolated
}

/// Whether execution of this scope is deferred relative to its surrounding statements.
fn execution(tree: &Tree<'_>, scope: Position) -> ScopeExecution {
    if matches!(
        kind(tree, scope),
        Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef | Node::Lambda | Node::GeneratorExp)
    ) {
        ScopeExecution::Deferred
    } else {
        ScopeExecution::Immediate
    }
}

/// An assignment's right side is evaluated before its targets are bound.
fn availability(tree: &Tree<'_>, binding: Position, occurrence: Position) -> BindingAvailability {
    let mut above = binding;
    while above != parent(tree, above) {
        if matches!(
            kind(tree, above),
            Kind::Node(Node::Assign | Node::AnnAssign | Node::NamedExpr)
        ) || (above == binding
            && matches!(
                kind(tree, above),
                Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef | Node::ClassDef)
            ))
        {
            return if occurrence > last(tree, above) || occurrence < above {
                BindingAvailability::Available
            } else {
                BindingAvailability::Unavailable
            };
        }
        if matches!(
            kind(tree, above),
            Kind::Node(
                Node::Module
                    | Node::FunctionDef
                    | Node::AsyncFunctionDef
                    | Node::ClassDef
                    | Node::Lambda
            )
        ) {
            break;
        }
        above = parent(tree, above);
    }
    BindingAvailability::Available
}

/// The lexical declaration of a name expression. Function locals shadow outer names even before
/// assignment. Class namespaces are visible in their own bodies but are not enclosing lexical
/// scopes for methods, comprehensions, or nested classes. Delayed function bodies can refer to
/// module declarations written later. Builtins and star imports have no local declaration here.
#[must_use]
pub(crate) fn resolve(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    name: Symbol,
) -> Option<Position> {
    bound(tree, model);
    let all = model.bindings.borrow();
    let initial = enclosing(tree, at);
    let mut scope = initial;
    let mut delayed = false;
    loop {
        let lookup = redirected(tree, model, scope, name);
        delayed |= lookup != scope && (execution(tree, scope) == ScopeExecution::Deferred);
        let class = kind(tree, lookup) == Kind::Node(Node::ClassDef);
        let key = BindingKey {
            scope: lookup,
            name,
        };
        let start = all.partition_point(|binding| binding.key < key);
        let end = all.partition_point(|binding| binding.key <= key);
        if (!class || scope == initial) && start < end {
            let bindings = all.get(start..end)?;
            let lexical = matches!(
                kind(tree, lookup),
                Kind::Node(
                    Node::FunctionDef
                        | Node::AsyncFunctionDef
                        | Node::Lambda
                        | Node::ListComp
                        | Node::SetComp
                        | Node::DictComp
                        | Node::GeneratorExp
                )
            );
            if let Some(found) = bindings.iter().rev().find(|binding| {
                evaluation_reach(tree, binding.position, at) == EvaluationReach::Shared
                    && (delayed || binding.position < at)
                    && (delayed
                        || lexical
                        || availability(tree, binding.position, at)
                            == BindingAvailability::Available)
            }) {
                return Some(found.position);
            }
            if lexical
                && let Some(binding) = bindings.iter().find(|binding| {
                    evaluation_reach(tree, binding.position, at) == EvaluationReach::Shared
                })
            {
                return Some(binding.position);
            }
        }
        delayed |= execution(tree, scope) == ScopeExecution::Deferred;
        if lookup == Position(0) {
            return None;
        }
        scope = enclosing(tree, scope);
    }
}
