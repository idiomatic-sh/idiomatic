//! Lexical name resolution, independent of any lint policy. Resolution associates a read with a
//! declaration; it does not assert that control flow has initialized that declaration.

use ryo::languages::{self, Position, Symbol, Tree};

use crate::domain::definitions::service::definition;
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::{enclosing, redirected};
use crate::domain::syntax::service::{kind, last, parent};
use crate::domain::syntax::types::Kind;

/// Index each declaration once, after scope redirects have been collected.
pub(crate) fn index(tree: &Tree<'_>, model: &mut Declarations) {
    for at in languages::positions(tree.elements) {
        if let Some(found) = definition(tree, model, at) {
            model
                .bindings
                .entry((found.owner, found.name))
                .or_default()
                .push(at);
        }
    }
}

/// Whether execution of this scope is deferred relative to its surrounding statements.
fn deferred(tree: &Tree<'_>, scope: Position) -> bool {
    matches!(
        kind(tree, scope),
        Kind::Node(Node::FunctionDef | Node::Lambda | Node::GeneratorExp)
    )
}

/// An assignment's right side is evaluated before its targets are bound.
fn initialized(tree: &Tree<'_>, binding: Position, occurrence: Position) -> bool {
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
            return occurrence > last(tree, above) || occurrence < above;
        }
        if matches!(
            kind(tree, above),
            Kind::Node(Node::Module | Node::FunctionDef | Node::ClassDef | Node::Lambda)
        ) {
            break;
        }
        above = parent(tree, above);
    }
    true
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
        delayed |= lookup != scope && deferred(tree, scope);
        let class = kind(tree, lookup) == Kind::Node(Node::ClassDef);
        if (!class || scope == initial)
            && let Some(bindings) = model.bindings.get(&(lookup, name))
        {
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
                (delayed || **binding < at)
                    && (delayed || lexical || initialized(tree, **binding, at))
            }) {
                return Some(*found);
            }
            if lexical {
                return bindings.first().copied();
            }
        }
        delayed |= deferred(tree, scope);
        if lookup == Position(0) {
            return None;
        }
        scope = enclosing(tree, scope);
    }
}
