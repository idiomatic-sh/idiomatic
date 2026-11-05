//! The `names` service: free functions named by verb over the `ESTree` projection, with no IO and no ports. It says which names a
//! declaration binds, of what kind it is, whether an occurrence of a name reads it, what a string literal is bound to, and what a
//! class inherits. Each function selects the shapes that carry its name; every other shape has none.

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "each name operation selects the syntax shapes that carry the name; other shapes have no such static name, and the adapter's schemas map the parser's vocabulary exhaustively"
)]

use super::constants::{CONSTANT, EXTENDS};
use super::types::{Assignment, Reference};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{COMPUTED_OPEN, PARAMETERS_OPEN};
use crate::domain::syntax::service::{kind, nodes, parameters, parent, spelling};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Tree};

/// The first direct child of `at` spelled `written`, a delimiter or a keyword of the node itself and none of a child's.
fn token(tree: &Tree<'_>, at: Position, written: languages::NameText<'_>) -> Option<Position> {
    languages::children(tree.elements, at).find(|child| spelling(tree, *child) == written)
}

/// The identifier a declaration names itself with: the first one written before `before`, or the first at all.
fn name_before(tree: &Tree<'_>, at: Position, before: Option<Position>) -> Option<Position> {
    nodes(tree, at).find(|child| {
        kind(tree, *child) == Kind::Node(Node::Identifier)
            && before.is_none_or(|before| {
                languages::start_at(tree.elements, *child)
                    < languages::start_at(tree.elements, before)
            })
    })
}

/// The names a pattern at `at` binds, without its property keys or its default values: the pattern itself when it is a name.
pub(crate) fn bindings<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Position> + 'a {
    (at.0..=languages::last_at(tree.elements, at).0)
        .map(Position)
        .filter(move |candidate| {
            if !matches!(
                kind(tree, *candidate),
                Kind::Node(Node::Identifier | Node::PrivateIdentifier | Node::Literal)
            ) {
                return false;
            }
            let mut child = *candidate;
            while child != at {
                let enclosing = parent(tree, child);
                let binds = match kind(tree, enclosing) {
                    Kind::Node(Node::RestElement | Node::AssignmentPattern) => {
                        nodes(tree, enclosing).next() == Some(child)
                    }
                    Kind::Node(Node::Property) => nodes(tree, enclosing).last() == Some(child),
                    Kind::Node(Node::ArrayPattern | Node::ObjectPattern) => true,
                    _ => false,
                };
                if !binds {
                    return false;
                }
                child = enclosing;
            }
            true
        })
}

/// The names the declaration at `at` binds: a function's or a class's name, each name a declarator's pattern binds, an import's
/// local name, a method's or a field's key when it is written, not computed, a statement's label.
pub(crate) fn declared<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Position> + 'a {
    let named = match kind(tree, at) {
        Kind::Node(
            Node::ImportSpecifier | Node::ImportDefaultSpecifier | Node::ImportNamespaceSpecifier,
        ) => nodes(tree, at).last(),
        Kind::Node(Node::FunctionDeclaration | Node::FunctionExpression) => name_before(
            tree,
            at,
            token(tree, at, languages::NameText(PARAMETERS_OPEN)),
        ),
        Kind::Node(Node::ClassDeclaration | Node::ClassExpression) => {
            name_before(tree, at, token(tree, at, languages::NameText(EXTENDS)))
        }
        Kind::Node(Node::MethodDefinition | Node::PropertyDefinition)
            if token(tree, at, languages::NameText(COMPUTED_OPEN)).is_some() =>
        {
            None
        }
        _ => nodes(tree, at).next(),
    };
    named
        .into_iter()
        .flat_map(move |child| bindings(tree, child))
}

/// The shared kind of the declaration at `at`.
#[must_use]
pub(crate) fn declaration_kind(tree: &Tree<'_>, at: Position) -> languages::DeclarationKind {
    use languages::DeclarationKind as Declaration;
    match kind(tree, at) {
        Kind::Node(Node::FunctionDeclaration) => Declaration::Function,
        Kind::Node(Node::ClassDeclaration) => Declaration::Class,
        Kind::Node(
            Node::ImportSpecifier | Node::ImportDefaultSpecifier | Node::ImportNamespaceSpecifier,
        ) => Declaration::Import,
        Kind::Node(Node::MethodDefinition) => Declaration::Method,
        Kind::Node(Node::PropertyDefinition) => Declaration::Property,
        Kind::Node(Node::LabeledStatement) => Declaration::Label,
        Kind::Node(Node::VariableDeclarator)
            if languages::children(tree.elements, parent(tree, at)).any(|child| {
                kind(tree, child) == Kind::Token(Token::Keyword)
                    && spelling(tree, child).0 == CONSTANT
            }) =>
        {
            Declaration::Constant
        }
        _ => Declaration::Variable,
    }
}

/// Whether the name at `at` is read where it occurs: a binding, an import's names, a part of a static member chain or a static
/// key, and the names a declaration or a parameter binds are not.
#[must_use]
pub(crate) fn referenced(tree: &Tree<'_>, at: Position) -> Reference {
    let enclosing = parent(tree, at);
    let parent_kind = kind(tree, enclosing);
    if matches!(
        parent_kind,
        Kind::Node(
            Node::ImportSpecifier | Node::ImportDefaultSpecifier | Node::ImportNamespaceSpecifier
        )
    ) {
        return Reference::Binding;
    }
    // A static member is read as one dotted name, the chain it is part of; a computed one names nothing static, so what it selects
    // from and the key it selects with are each read on their own.
    if parent_kind == Kind::Node(Node::MemberExpression)
        && token(tree, enclosing, languages::NameText(COMPUTED_OPEN)).is_none()
    {
        return Reference::Binding;
    }
    if matches!(
        parent_kind,
        Kind::Node(Node::Property | Node::MethodDefinition | Node::PropertyDefinition)
    ) && nodes(tree, enclosing).last() != Some(at)
        && token(tree, enclosing, languages::NameText(COMPUTED_OPEN)).is_none()
    {
        return Reference::Binding;
    }
    let mut ancestor = enclosing;
    loop {
        if matches!(
            kind(tree, ancestor),
            Kind::Node(
                Node::VariableDeclarator
                    | Node::FunctionDeclaration
                    | Node::FunctionExpression
                    | Node::ClassDeclaration
                    | Node::ClassExpression
                    | Node::MethodDefinition
                    | Node::PropertyDefinition
                    | Node::LabeledStatement
            )
        ) && declared(tree, ancestor).any(|binding| binding == at)
        {
            return Reference::Binding;
        }
        let callable = matches!(
            kind(tree, ancestor),
            Kind::Node(
                Node::FunctionDeclaration
                    | Node::FunctionExpression
                    | Node::ArrowFunctionExpression
            )
        );
        if callable
            && parameters(tree, ancestor)
                .any(|parameter| bindings(tree, parameter).any(|binding| binding == at))
        {
            return Reference::Binding;
        }
        if ancestor == parent(tree, ancestor) {
            break;
        }
        ancestor = parent(tree, ancestor);
    }
    Reference::Use
}

/// The name a declarator or an assignment at `at` binds and the string literal it binds it to; none when the value is no string.
#[must_use]
pub(crate) fn assignment(tree: &Tree<'_>, at: Position) -> Option<Assignment> {
    let mut children = nodes(tree, at);
    let binding = children.next()?;
    let value = children.last()?;
    languages::children(tree.elements, value)
        .any(|child| kind(tree, child) == Kind::Token(Token::String))
        .then_some(Assignment { binding, value })
}

/// What the class at `at` extends: the expression after its `extends`, before its body.
pub(crate) fn bases<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    let extends = token(tree, at, languages::NameText(EXTENDS))
        .map(|child| languages::end_at(tree.elements, child));
    nodes(tree, at).filter(move |child| {
        extends.is_some_and(|from| languages::start_at(tree.elements, *child) >= from)
            && kind(tree, *child) != Kind::Node(Node::ClassBody)
    })
}
