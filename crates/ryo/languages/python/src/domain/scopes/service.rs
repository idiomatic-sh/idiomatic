//! The `scopes` service: free functions named by verb over the domain's types, with no IO and no ports. A scope is a function, a
//! class, a lambda or a comprehension; the module is the scope of everything outside them. What a definition's decorators, defaults,
//! annotations and bases say is evaluated in the scope around it, and so is the first iterable of a comprehension.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use ryo::languages::{self, Position, Symbol, Tree};

use super::types::{CategoryIndex, Context, Evaluation, Scopeness, Targeting};
use crate::domain::model::service::redirect_of;
use crate::domain::model::types::{BindingRedirect, Declarations, ScopeDeclaration};
use crate::domain::nodes::types::Category;
use crate::domain::syntax::service::{category, is_root, kind, name_of, nodes, nodes_of, parent};
use crate::domain::syntax::types::{Kind, Rootness};

/// Whether `kind` opens a scope of its own.
#[must_use]
pub(crate) const fn scopeness(kind: Kind) -> Scopeness {
    if matches!(
        kind,
        Kind::Node(
            Node::FunctionDef
                | Node::AsyncFunctionDef
                | Node::ClassDef
                | Node::Lambda
                | Node::ListComp
                | Node::SetComp
                | Node::DictComp
                | Node::GeneratorExp
        )
    ) {
        Scopeness::Scope
    } else {
        Scopeness::Inner
    }
}

/// The place of `child` among the nodes of `of` that are of the part of the grammar `wanted`, from zero.
#[must_use]
pub(crate) fn place_among(
    tree: &Tree<'_>,
    of: Position,
    child: Position,
    wanted: Category,
) -> Option<CategoryIndex> {
    nodes_of(tree, of, wanted)
        .position(|each| each == child)
        .map(CategoryIndex)
}

/// The `n`-th node of `of` of the part of the grammar `wanted`, from zero.
#[must_use]
pub(crate) fn nth(
    tree: &Tree<'_>,
    of: Position,
    wanted: Category,
    n: CategoryIndex,
) -> Option<Position> {
    nodes_of(tree, of, wanted).nth(n.0)
}

/// Whether `child`, a child of the scope `scope`, is evaluated inside it.
fn inside(tree: &Tree<'_>, scope: Position, child: Position) -> Scopeness {
    let of = kind(tree, scope);
    let within = if matches!(
        of,
        Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef | Node::ClassDef)
    ) {
        category(tree, child) == Category::Statement
            || kind(tree, child) == Kind::Extra(Extra::TypeParams)
    } else if of == Kind::Node(Node::Lambda) {
        category(tree, child) == Category::Expression
    } else {
        scopeness(of) == Scopeness::Scope
    };
    if within {
        Scopeness::Scope
    } else {
        Scopeness::Inner
    }
}

/// Whether `child` is the iterable of the comprehension clause `clause`, and the clause the first of its comprehension: what is
/// evaluated in the scope around the comprehension.
fn evaluation(tree: &Tree<'_>, clause: Position, child: Position) -> Evaluation {
    if place_among(tree, clause, child, Category::Expression) == Some(CategoryIndex(1))
        && nodes(tree, parent(tree, clause))
            .find(|each| kind(tree, *each) == Kind::Node(Node::Comprehension))
            == Some(clause)
    {
        Evaluation::Around
    } else {
        Evaluation::Within
    }
}

/// The scope the element at `at` is evaluated in: the nearest function, class, lambda or comprehension that holds it, or the root.
#[must_use]
pub(crate) fn enclosing(tree: &Tree<'_>, at: Position) -> Position {
    let mut child = at;
    let mut above = parent(tree, at);
    loop {
        if is_root(tree, child) == Rootness::Root || is_root(tree, above) == Rootness::Root {
            return above;
        }
        if kind(tree, above) == Kind::Node(Node::Comprehension)
            && evaluation(tree, above, child) == Evaluation::Around
        {
            child = parent(tree, above);
            above = parent(tree, child);
            continue;
        }
        if scopeness(kind(tree, above)) == Scopeness::Scope
            && inside(tree, above, child) == Scopeness::Scope
        {
            return above;
        }
        child = above;
        above = parent(tree, above);
    }
}

/// The scope `at` is evaluated in, as a node id: the owner a row names.
#[must_use]
pub(crate) fn owner(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    languages::node_id(tree.file, enclosing(tree, at))
}

/// The scope that encloses the scope at `at`, as a node id: the module of the file encloses itself.
#[must_use]
pub(crate) fn scope_owner(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    if is_root(tree, at) == Rootness::Root {
        languages::node_id(tree.file, at)
    } else {
        owner(tree, at)
    }
}

/// The nearest function or lambda the element at `at` is in, or the root.
#[must_use]
pub(crate) fn function_of(tree: &Tree<'_>, at: Position) -> Position {
    let mut scope = enclosing(tree, at);
    while !matches!(
        kind(tree, scope),
        Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef | Node::Lambda)
    ) && is_root(tree, scope) == Rootness::Inner
    {
        scope = enclosing(tree, scope);
    }
    scope
}

/// The nearest function around the scope `scope`, or the root: where `nonlocal` binds.
fn function_around(tree: &Tree<'_>, scope: Position) -> Position {
    let mut around = enclosing(tree, scope);
    while !matches!(
        kind(tree, around),
        Kind::Node(Node::FunctionDef | Node::AsyncFunctionDef)
    ) && is_root(tree, around) == Rootness::Inner
    {
        around = enclosing(tree, around);
    }
    around
}

/// Where the scope `scope` binds `name`: in itself, or where its `global` or `nonlocal` says.
#[must_use]
pub(crate) fn redirected(
    tree: &Tree<'_>,
    model: &Declarations,
    scope: Position,
    name: Symbol,
) -> Position {
    match redirect_of(model, scope, name) {
        Some(BindingRedirect::Global) => Position(0),
        Some(BindingRedirect::Nonlocal) => function_around(tree, scope),
        None => scope,
    }
}

/// Whether the name expression at `at` is the target of an assignment expression, `(n := ...)`, which binds outside any comprehension.
fn target(tree: &Tree<'_>, at: Position) -> Targeting {
    let above = parent(tree, at);
    if kind(tree, above) == Kind::Node(Node::NamedExpr)
        && place_among(tree, above, at, Category::Expression) == Some(CategoryIndex(0))
    {
        Targeting::AssignmentExpression
    } else {
        Targeting::Other
    }
}

/// Where the name `name` the element at `at` binds is bound: its scope, out of any comprehension for an assignment expression, then
/// where `global` or `nonlocal` sends it.
#[must_use]
pub(crate) fn binding_scope(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    name: Symbol,
) -> Position {
    let mut scope = enclosing(tree, at);
    if target(tree, at) == Targeting::AssignmentExpression {
        while matches!(
            kind(tree, scope),
            Kind::Node(Node::ListComp | Node::SetComp | Node::DictComp | Node::GeneratorExp)
        ) {
            scope = enclosing(tree, scope);
        }
    }
    redirected(tree, model, scope, name)
}

/// What the expression at `at` does with what it names: in a target of an assignment, a loop, a `with`, a comprehension or an
/// assignment expression it binds, in `del` it deletes, anywhere else it reads. A tuple, a list or a starred expression passes its place
/// on to the elements it holds.
#[must_use]
pub(crate) fn context(tree: &Tree<'_>, at: Position) -> Context {
    let mut child = at;
    let mut above = parent(tree, at);
    while matches!(
        kind(tree, above),
        Kind::Node(Node::Tuple | Node::List | Node::Starred)
    ) && is_root(tree, above) == Rootness::Inner
    {
        child = above;
        above = parent(tree, above);
    }
    let place = place_among(tree, above, child, Category::Expression);
    let count = nodes_of(tree, above, Category::Expression).count();
    let of = kind(tree, above);
    if of == Kind::Node(Node::Delete) {
        return Context::Del;
    }
    let binds = if of == Kind::Node(Node::Assign) {
        place.is_some_and(|among| among.0 + 1 < count)
    } else if matches!(
        of,
        Kind::Node(
            Node::AugAssign
                | Node::AnnAssign
                | Node::For
                | Node::AsyncFor
                | Node::Comprehension
                | Node::NamedExpr
                | Node::TypeAlias
        )
    ) {
        place == Some(CategoryIndex(0))
    } else {
        of == Kind::Node(Node::Withitem) && place == Some(CategoryIndex(1))
    };
    if binds { Context::Store } else { Context::Load }
}

#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only `global` and `nonlocal` declare where a name is bound; every other kind declares nothing"
)]
/// Refills the scope redirects of the file; declaration indexing is a separate phase.
pub(crate) fn declare(tree: &Tree<'_>, model: &mut Declarations) {
    model.declared.clear();
    for at in languages::positions(tree.elements) {
        let redirect = match kind(tree, at) {
            Kind::Node(Node::Global) => BindingRedirect::Global,
            Kind::Node(Node::Nonlocal) => BindingRedirect::Nonlocal,
            _ => continue,
        };
        let scope = enclosing(tree, at);
        for identifier in nodes(tree, at) {
            if let Some(name) = name_of(tree, identifier) {
                model.declared.push(ScopeDeclaration {
                    scope,
                    name,
                    redirect,
                });
            }
        }
    }
}
