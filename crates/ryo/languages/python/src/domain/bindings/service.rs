//! Lexical name resolution, independent of any lint policy. Resolution associates a read with a
//! declaration; it does not assert that control flow has initialized that declaration.

use ryo::languages::{self, Position, Symbol, Tree};

use super::types::{BindingAvailability, ScopeExecution};
use crate::domain::definitions::service::definition;
use crate::domain::model::types::{BindingKey, Declarations, LexicalBinding};
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::{enclosing, redirected};
use crate::domain::syntax::service::{kind, last, parent};
use crate::domain::syntax::types::Kind;

/// Index each declaration once, after scope redirects have been collected.
pub(crate) fn index(tree: &Tree<'_>, model: &mut Declarations) {
    model.bindings.clear();
    for at in languages::positions(tree.elements) {
        if let Some(found) = definition(tree, model, at) {
            model.bindings.push(LexicalBinding {
                key: BindingKey {
                    scope: found.owner,
                    name: found.name,
                },
                position: at,
            });
        }
    }
    model.bindings.sort_unstable();
}

/// Whether execution of this scope is deferred relative to its surrounding statements.
fn execution(tree: &Tree<'_>, scope: Position) -> ScopeExecution {
    if matches!(
        kind(tree, scope),
        Kind::Node(Node::FunctionDef | Node::Lambda | Node::GeneratorExp)
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
                Kind::Node(Node::FunctionDef | Node::ClassDef)
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
            Kind::Node(Node::Module | Node::FunctionDef | Node::ClassDef | Node::Lambda)
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
        let start = model.bindings.partition_point(|binding| binding.key < key);
        let end = model.bindings.partition_point(|binding| binding.key <= key);
        if (!class || scope == initial) && start < end {
            let bindings = &model.bindings[start..end];
            let lexical = matches!(
                kind(tree, lookup),
                Kind::Node(
                    Node::FunctionDef
                        | Node::Lambda
                        | Node::ListComp
                        | Node::SetComp
                        | Node::DictComp
                        | Node::GeneratorExp
                )
            );
            if let Some(found) = bindings.iter().rev().find(|binding| {
                (delayed || binding.position < at)
                    && (delayed
                        || lexical
                        || availability(tree, binding.position, at)
                            == BindingAvailability::Available)
            }) {
                return Some(found.position);
            }
            if lexical {
                return bindings.first().map(|binding| binding.position);
            }
        }
        delayed |= execution(tree, scope) == ScopeExecution::Deferred;
        if lookup == Position(0) {
            return None;
        }
        scope = enclosing(tree, scope);
    }
}
