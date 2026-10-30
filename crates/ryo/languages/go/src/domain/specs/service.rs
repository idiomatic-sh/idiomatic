//! The `specs` service: free functions named by verb over the domain's types, with no IO and no ports.

use ryo::languages::{self, Count, Position, Symbol, Tree};

use super::types::{Declares, Underlying};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, nodes, parent, token, tokens};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What the declaration that holds the spec at `spec` declares, by its keyword.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a declaration is told by its keyword; every other token declares nothing"
)]
pub(crate) fn declares(tree: &Tree<'_>, spec: Position) -> Option<Declares> {
    tokens(tree, parent(tree, spec)).find_map(|each| match each.token {
        Token::Const => Some(Declares::Constants),
        Token::Var => Some(Declares::Variables),
        Token::Type => Some(Declares::Types),
        Token::Import => Some(Declares::Imports),
        _ => None,
    })
}

/// How many names the `ValueSpec` at `spec` declares: one more than the commas before its `=`.
fn name_count(tree: &Tree<'_>, spec: Position) -> Count {
    Count(
        1 + tokens(tree, spec)
            .take_while(|each| each.token != Token::Assign)
            .filter(|each| each.token == Token::Comma)
            .count(),
    )
}

/// The names the `ValueSpec` at `spec` declares, in order.
pub(crate) fn names<'a>(tree: &'a Tree<'_>, spec: Position) -> impl Iterator<Item = Position> + 'a {
    nodes(tree, spec).take(name_count(tree, spec).0)
}

/// The type the `ValueSpec` at `spec` gives its names: the element after them, when it is before the `=`.
#[must_use]
pub(crate) fn value_type(tree: &Tree<'_>, spec: Position) -> Option<Position> {
    let given = nodes(tree, spec).nth(name_count(tree, spec).0)?;
    match token(tree, spec, Token::Assign) {
        Some(assign) if assign < given => None,
        Some(_) | None => Some(given),
    }
}

/// The type the `TypeSpec` at `spec` declares: its last element.
#[must_use]
pub(crate) fn declared_type(tree: &Tree<'_>, spec: Position) -> Option<Position> {
    nodes(tree, spec).last()
}

/// What the `TypeSpec` at `spec` declares.
#[must_use]
pub(crate) fn underlying(tree: &Tree<'_>, spec: Position) -> Underlying {
    if token(tree, spec, Token::Assign).is_some() {
        return Underlying::Alias;
    }
    match declared_type(tree, spec).map(|each| kind(tree, each)) {
        Some(Kind::Node(Node::StructType)) => Underlying::Structure,
        Some(Kind::Node(Node::InterfaceType)) => Underlying::Interface,
        Some(Kind::Node(_) | Kind::Token(_) | Kind::Extra(_)) | None => Underlying::Other,
    }
}

/// The name of the `TypeSpec` or the `ImportSpec` at `spec`: its first element when that is an identifier.
#[must_use]
pub(crate) fn name(tree: &Tree<'_>, spec: Position) -> Option<Position> {
    nodes(tree, spec)
        .next()
        .filter(|each| kind(tree, *each) == Kind::Node(Node::Ident))
}

/// The name the `ImportSpec` at `spec` gives the package: the identifier before the path.
#[must_use]
pub(crate) fn import_name(tree: &Tree<'_>, spec: Position) -> Option<Position> {
    let mut each = nodes(tree, spec);
    let first = each.next()?;
    (each.next().is_some() && kind(tree, first) == Kind::Node(Node::Ident)).then_some(first)
}

/// The path the `ImportSpec` at `spec` imports: its string literal.
#[must_use]
pub(crate) fn import_path(tree: &Tree<'_>, spec: Position) -> Option<Position> {
    nodes(tree, spec).find(|each| kind(tree, *each) == Kind::Node(Node::BasicLit))
}

/// The identifier at `ident`, when it is written as a type: the type of a field, a pointer, an array, a map, a channel, a variadic
/// parameter, a generic type or a declared type, of a composite literal or a type assertion.
#[must_use]
pub(crate) fn written_as_type(tree: &Tree<'_>, ident: Position) -> Option<Position> {
    let above = parent(tree, ident);
    let mut siblings = nodes(tree, above);
    let first = siblings.next();
    let last = nodes(tree, above).last();
    let typed = match kind(tree, above) {
        Kind::Node(
            Node::StarExpr | Node::MapType | Node::ChanType | Node::Ellipsis | Node::ArrayType,
        ) => Some(ident),
        Kind::Node(Node::IndexExpr | Node::IndexListExpr | Node::CompositeLit) => first,
        Kind::Node(Node::Field | Node::TypeAssertExpr) => last,
        Kind::Node(Node::TypeSpec) => last.filter(|_| first != last),
        Kind::Node(Node::ValueSpec) => value_type(tree, above),
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
    };
    typed.filter(|each| *each == ident)
}

/// The `type` spec of the declaration `declaration` that is named `name`.
fn spec_named(tree: &Tree<'_>, declaration: Position, name: Symbol) -> Option<Position> {
    nodes(tree, declaration).find(|each| {
        kind(tree, *each) == Kind::Node(Node::TypeSpec) && name_of_spec(tree, *each) == Some(name)
    })
}

/// The type declaration of the scope `scope` that is named `name`, when it declares one: the `type` declarations of a file or a block.
fn declares_type(tree: &Tree<'_>, scope: Position, name: Symbol) -> Option<Position> {
    nodes(tree, scope).find_map(|each| match kind(tree, each) {
        Kind::Node(Node::GenDecl) => spec_named(tree, each, name),
        Kind::Node(Node::DeclStmt) => nodes(tree, each)
            .filter(|inner| kind(tree, *inner) == Kind::Node(Node::GenDecl))
            .find_map(|inner| spec_named(tree, inner, name)),
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
    })
}

/// The name the `TypeSpec` at `spec` declares.
fn name_of_spec(tree: &Tree<'_>, spec: Position) -> Option<Symbol> {
    crate::domain::names::service::name_of(tree, name(tree, spec)?)
}

/// The `type` declaration that binds `name` where the element at `at` is written: the innermost file or block that declares it.
#[must_use]
pub(crate) fn binding_of(tree: &Tree<'_>, at: Position, name: Symbol) -> Option<Position> {
    let mut above = at;
    while parent(tree, above) != above {
        above = parent(tree, above);
        if matches!(kind(tree, above), Kind::Node(Node::File | Node::BlockStmt))
            && let Some(found) = declares_type(tree, above, name)
        {
            return Some(found);
        }
    }
    None
}

/// What holds the element at `at`: the nearest type declaration or function it is written in, else the file.
#[must_use]
pub(crate) fn holder(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    let mut above = at;
    while parent(tree, above) != above {
        above = parent(tree, above);
        if matches!(
            kind(tree, above),
            Kind::Node(Node::TypeSpec | Node::FuncDecl)
        ) {
            return languages::node_id(tree.file, above);
        }
    }
    languages::node_id(tree.file, above)
}
