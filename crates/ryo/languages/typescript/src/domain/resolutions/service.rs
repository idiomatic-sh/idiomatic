//! The `resolutions` service: free functions named by verb that say which declaration binds a type name where it is written, with no IO
//! and no ports.

use crate::domain::names::service::{declared, identifier, nodes};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::{kind, parent},
    types::Kind,
};
use ryo::languages::{self, Position, Symbol, Symbols, Tree};

/// The one name the type reference at `at` writes: a reference to an identifier, with no qualifier before it. The identifier is the
/// first node of the reference.
#[must_use]
pub(crate) fn plain_type_name(tree: &Tree<'_>, at: Position) -> Option<Position> {
    if kind(tree, at) != Kind::Node(Node::TSTypeReference) {
        return None;
    }
    nodes(tree, at)
        .next()
        .filter(|name| kind(tree, *name) == Kind::Node(Node::Identifier))
}

/// The declaration that `statement` is, looking through an `export`.
fn declaration_of(tree: &Tree<'_>, statement: Position) -> Position {
    if matches!(
        kind(tree, statement),
        Kind::Node(Node::ExportNamedDeclaration | Node::ExportDefaultDeclaration)
    ) {
        nodes(tree, statement).next().unwrap_or(statement)
    } else {
        statement
    }
}

/// What the scope `scope` binds `name` to, when it does: a class, an interface, a type alias or an enum it declares, a name an import
/// of it brings in, or a type parameter of the declaration it is.
fn binds(
    tree: &Tree<'_>,
    symbols: &mut Symbols,
    scope: Position,
    name: Symbol,
) -> Option<Position> {
    let mut called = |declaration: Position| {
        declared(tree, declaration).any(|binding| identifier(tree, symbols, binding) == name)
    };
    if let Some(list) = nodes(tree, scope)
        .find(|each| kind(tree, *each) == Kind::Node(Node::TSTypeParameterDeclaration))
        && let Some(parameter) = nodes(tree, list)
            .find(|each| kind(tree, *each) == Kind::Node(Node::TSTypeParameter) && called(*each))
    {
        return Some(parameter);
    }
    if !matches!(
        kind(tree, scope),
        Kind::Node(Node::Program | Node::TSModuleBlock | Node::BlockStatement)
    ) {
        return None;
    }
    for statement in nodes(tree, scope) {
        let declaration = declaration_of(tree, statement);
        match kind(tree, declaration) {
            Kind::Node(
                Node::ClassDeclaration
                | Node::TSInterfaceDeclaration
                | Node::TSTypeAliasDeclaration
                | Node::TSEnumDeclaration,
            ) if called(declaration) => return Some(declaration),
            Kind::Node(Node::ImportDeclaration) => {
                if let Some(specifier) = nodes(tree, declaration).find(|each| {
                    matches!(
                        kind(tree, *each),
                        Kind::Node(
                            Node::ImportSpecifier
                                | Node::ImportDefaultSpecifier
                                | Node::ImportNamespaceSpecifier
                        )
                    ) && called(*each)
                }) {
                    return Some(specifier);
                }
            }
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => {}
        }
    }
    None
}

/// The declaration, import or type parameter that binds `name` where the element at `at` is written: the innermost scope wins.
#[must_use]
pub(crate) fn binding_of(
    tree: &Tree<'_>,
    symbols: &mut Symbols,
    at: Position,
    name: Symbol,
) -> Option<Position> {
    let mut above = at;
    while parent(tree, above) != above {
        above = parent(tree, above);
        if let Some(found) = binds(tree, symbols, above, name) {
            return Some(found);
        }
    }
    None
}

/// What holds the element at `at`: the nearest class, interface, type alias, enum, function or method it is written in, else the file.
#[must_use]
pub(crate) fn holder(tree: &Tree<'_>, at: Position) -> languages::NodeId {
    let mut above = at;
    while parent(tree, above) != above {
        above = parent(tree, above);
        if matches!(
            kind(tree, above),
            Kind::Node(
                Node::ClassDeclaration
                    | Node::TSInterfaceDeclaration
                    | Node::TSTypeAliasDeclaration
                    | Node::TSEnumDeclaration
                    | Node::FunctionDeclaration
                    | Node::MethodDefinition
            )
        ) {
            return languages::node_id(tree.file, above);
        }
    }
    languages::node_id(tree.file, above)
}
