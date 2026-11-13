//! The placing of the adapter: spelled, setting, place, markers_before and more.

use super::super::constants::PARENTHESES;
use super::super::types::{
    Field, KindRole, Lastness, Marker, MarkerRole, Operands, Ordinal, Precedence, Returning,
    Setting, Sharing, Standing,
};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{is_token, kind, parent};
use crate::domain::syntax::types::{Kind, Leaf};
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Tree};

/// The bytes of the element at `at`, as the file spells them.
pub(in super::super) fn spelled<'a>(tree: &Tree<'a>, at: Position) -> languages::NameText<'a> {
    let from = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let to = usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX);
    languages::NameText(
        tree.source
            .0
            .get(from..to)
            // the bytes of a node that are not text have no text, which the reader takes as an empty name: the row keeps its place either way
            .map(|bytes| std::str::from_utf8(bytes).unwrap_or_default())
            .unwrap_or_default(),
    )
}

/// Whether a child of this kind is held in a property of its own whatever its place: a decorator, an annotation, a type parameter list
/// or a type argument list.
pub(in super::super) const fn setting(child: Kind) -> Setting {
    if matches!(
        child,
        Kind::Node(
            Node::Decorator
                | Node::TSTypeAnnotation
                | Node::TSTypeParameterDeclaration
                | Node::TSTypeParameterInstantiation
        )
    ) {
        Setting::Aside
    } else {
        Setting::Placed
    }
}

/// Where `at` stands among the nodes under its parent, `aside` ones left out, and whether it is the last of them.
pub(in super::super) fn place(tree: &Tree<'_>, at: Position) -> Standing {
    let placed: Vec<Position> = languages::children(tree.elements, parent(tree, at))
        .filter(|each| {
            let each = *each;
            matches!(kind(tree, each), Kind::Node(_))
                && setting(kind(tree, each)) == Setting::Placed
        })
        .collect();
    let ordinal = placed
        .iter()
        .position(|each| *each == at)
        .unwrap_or_default();
    Standing {
        ordinal: Ordinal(ordinal),
        lastness: if placed.last() == Some(&at) {
            Lastness::Last
        } else {
            Lastness::NotLast
        },
    }
}

/// The tokens among the children of the parent of `at` that come before it and are spelled `marker`.
pub(in super::super) fn markers_before<'a>(
    tree: &'a Tree<'_>,
    at: Position,
    marker: Marker,
) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, parent(tree, at))
        .take_while(move |each| *each != at)
        .filter(move |each| {
            let each = *each;
            is_token(tree, each) == Leaf::Token && spelled(tree, each).0 == marker.0
        })
}

/// Whether a token spelled `marker` comes before `at` among the children of its parent.
pub(in super::super) fn after(tree: &Tree<'_>, at: Position, marker: Marker) -> Precedence {
    if markers_before(tree, at, marker).next().is_some() {
        Precedence::Preceded
    } else {
        Precedence::Unpreceded
    }
}

/// How many tokens spelled `marker` come before `at` among the children of its parent: the place of the clause they open.
pub(in super::super) fn count_before(tree: &Tree<'_>, at: Position, marker: Marker) -> Ordinal {
    Ordinal(markers_before(tree, at, marker).count())
}

/// Whether the nodes under the parent of `at` all start where it does: a shorthand that holds one name in two properties
/// (`export { a }`).
pub(in super::super) fn sharing(tree: &Tree<'_>, at: Position) -> Sharing {
    let start = languages::start_at(tree.elements, at);
    if languages::children(tree.elements, parent(tree, at))
        .filter(|each| matches!(kind(tree, *each), Kind::Node(_)))
        .all(|each| languages::start_at(tree.elements, each) == start)
    {
        Sharing::Shared
    } else {
        Sharing::Apart
    }
}

/// The role a node held by its place has: the `ordinal`-th of `roles`, the last of them repeated.
pub(in super::super) fn nth(roles: &[Field], ordinal: Ordinal) -> Field {
    roles
        .get(ordinal.0)
        .or_else(|| roles.last())
        .copied()
        .unwrap_or_default()
}

/// The token that spells the operator of the expression at `holder`: the first token that is neither a comment nor a parenthesis,
/// after the left operand of a binary expression, anywhere in a unary or an update one; none for any other node.
pub(in super::super) fn operator_of(tree: &Tree<'_>, holder: Position) -> Option<Position> {
    let operands = match kind(tree, holder) {
        Kind::Node(
            Node::BinaryExpression | Node::LogicalExpression | Node::AssignmentExpression,
        ) => Operands::Binary,
        Kind::Node(Node::UnaryExpression | Node::UpdateExpression) => Operands::Unary,
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => return None,
    };
    let children: Vec<Position> = languages::children(tree.elements, holder).collect();
    // A token before the left operand of a binary expression is a parenthesis of the operand.
    let from = match operands {
        Operands::Binary => children
            .iter()
            .position(|each| matches!(kind(tree, *each), Kind::Node(_)))?
            .saturating_add(1),
        Operands::Unary => 0,
    };
    children.get(from..)?.iter().copied().find(|each| {
        let each = *each;
        matches!(kind(tree, each), Kind::Token(_))
            && !matches!(kind(tree, each), Kind::Token(Token::Block | Token::Line))
            && !PARENTHESES
                .iter()
                .any(|marker| spelled(tree, each).0 == marker.0)
    })
}

/// Whether a type annotation under a node of `holder` kind says what it returns.
pub(in super::super) const fn returning(holder: Node) -> Returning {
    if matches!(
        holder,
        Node::FunctionDeclaration
            | Node::FunctionExpression
            | Node::ArrowFunctionExpression
            | Node::TSDeclareFunction
            | Node::TSEmptyBodyFunctionExpression
            | Node::TSCallSignatureDeclaration
            | Node::TSConstructSignatureDeclaration
            | Node::TSFunctionType
            | Node::TSConstructorType
            | Node::TSMethodSignature
    ) {
        Returning::Returns
    } else {
        Returning::Holds
    }
}

/// The role a child of the kind `node` plays.
pub(in super::super) const fn kinded(node: Node, field: Field) -> KindRole {
    KindRole { node, field }
}

/// The role of the children after a token spelled `marker`.
pub(in super::super) const fn marked(marker: Marker, field: Field) -> MarkerRole {
    MarkerRole { marker, field }
}

/// The role of a child of `child` kind: the role paired with its kind in `kinds`, else `otherwise`.
pub(in super::super) fn by_kind(child: Kind, kinds: &[KindRole], otherwise: Field) -> Field {
    kinds
        .iter()
        .find(|each| child == Kind::Node(each.node))
        .map_or(otherwise, |each| each.field)
}

/// The role of `at` by the tokens before it: the role paired with the first of `markers` a token before it spells, else `otherwise`.
pub(in super::super) fn by_marker(
    tree: &Tree<'_>,
    at: Position,
    markers: &[MarkerRole],
    otherwise: Field,
) -> Field {
    markers
        .iter()
        .find(|each| after(tree, at, each.marker) == Precedence::Preceded)
        .map_or(otherwise, |each| each.field)
}
