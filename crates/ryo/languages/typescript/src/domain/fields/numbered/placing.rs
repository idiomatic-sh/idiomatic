//! The placing of the numbered writer: a child's place among its siblings, the tokens before it and what spells them.

use super::super::constants::PARENTHESES;
use super::super::types::{
    Field, KindRole, Lastness, Marker, MarkerRole, Operands, Ordinal, Precedence, Returning, Setting, Sharing, Standing,
};
use super::Family;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// The bytes of the sibling `at`, as the file spells them: a node's that are not text have no text.
pub(super) fn spelled<'a>(fam: &Family<'a>, at: usize) -> &'a str {
    let from = usize::try_from(fam.starts[at]).unwrap_or(usize::MAX);
    let to = usize::try_from(fam.ends[at]).unwrap_or(usize::MAX);
    fam.source.get(from..to).unwrap_or_default()
}

/// Whether a child of this kind is held in a property of its own whatever its place: a decorator, an annotation, a type parameter list
/// or a type argument list.
pub(super) const fn setting(child: Kind) -> Setting {
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

/// The siblings a place counts, in order: every node child but the ones `setting` holds aside.
fn placed_children<'a>(fam: &'a Family<'_>) -> impl Iterator<Item = usize> + 'a {
    (0..fam.kinds.len()).filter(move |&each| {
        matches!(fam.kinds[each], Kind::Node(_)) && setting(fam.kinds[each]) == Setting::Placed
    })
}

/// Where `at` stands among the nodes under its parent, `aside` ones left out, and whether it is the last of them.
pub(super) fn place(fam: &Family<'_>, at: usize) -> Standing {
    let mut ordinal = 0;
    let mut last = None;
    for (at_place, each) in placed_children(fam).enumerate() {
        if each == at {
            ordinal = at_place;
        }
        last = Some(each);
    }
    Standing { ordinal: Ordinal(ordinal), lastness: if last == Some(at) { Lastness::Last } else { Lastness::NotLast } }
}

/// The role of `at` by its place: the `ordinal`-th of `roles`, the last of them for every place from the last on.
pub(super) fn placed_nth(fam: &Family<'_>, at: usize, roles: &[Field]) -> Field {
    let bound = roles.len().saturating_sub(1);
    let ordinal = placed_children(fam).take(bound).position(|each| each == at).unwrap_or(bound);
    nth(roles, Ordinal(ordinal))
}

/// The tokens among the children spelled `marker`, in order.
fn marker_tokens<'a>(fam: &'a Family<'_>, marker: Marker) -> impl Iterator<Item = usize> + 'a {
    (0..fam.kinds.len()).filter(move |&each| is_leaf(fam.kinds[each]) && spelled(fam, each) == marker.0)
}

/// Whether the element is a token or an extra: a leaf.
fn is_leaf(kind: Kind) -> bool {
    !matches!(kind, Kind::Node(_))
}

/// Whether a token spelled `marker` comes before `at` among the children of its parent.
pub(super) fn after(fam: &Family<'_>, at: usize, marker: Marker) -> Precedence {
    if marker_tokens(fam, marker).next().is_some_and(|token| token < at) {
        Precedence::Preceded
    } else {
        Precedence::Unpreceded
    }
}

/// How many tokens spelled `marker` come before `at` among the children of its parent: the place of the clause they open.
pub(super) fn count_before(fam: &Family<'_>, at: usize, marker: Marker) -> Ordinal {
    Ordinal(marker_tokens(fam, marker).take_while(|token| *token < at).count())
}

/// Whether the nodes under the parent all start where `at` does: a shorthand that holds one name in two properties (`export { a }`).
pub(super) fn sharing(fam: &Family<'_>, at: usize) -> Sharing {
    let start = fam.starts[at];
    if (0..fam.kinds.len()).filter(|&each| matches!(fam.kinds[each], Kind::Node(_))).all(|each| fam.starts[each] == start) {
        Sharing::Shared
    } else {
        Sharing::Apart
    }
}

/// The role a node held by its place has: the `ordinal`-th of `roles`, the last of them repeated.
pub(super) fn nth(roles: &[Field], ordinal: Ordinal) -> Field {
    roles.get(ordinal.0).or_else(|| roles.last()).copied().unwrap_or(Field::None)
}

/// The token that spells the operator of the expression the family belongs to: the first token that is neither a comment nor a
/// parenthesis, after the left operand of a binary expression, anywhere in a unary or an update one; none for any other node.
pub(super) fn operator_of(fam: &Family<'_>) -> Option<usize> {
    let operands = match fam.holder {
        Node::BinaryExpression | Node::LogicalExpression | Node::AssignmentExpression => Operands::Binary,
        Node::UnaryExpression | Node::UpdateExpression => Operands::Unary,
        _ => return None,
    };
    // A token before the left operand of a binary expression is a parenthesis of the operand.
    let from = match operands {
        Operands::Binary => fam.kinds.iter().position(|each| matches!(each, Kind::Node(_)))?.saturating_add(1),
        Operands::Unary => 0,
    };
    (from..fam.kinds.len()).find(|&each| {
        matches!(fam.kinds[each], Kind::Token(_))
            && !matches!(fam.kinds[each], Kind::Token(Token::Block | Token::Line))
            && !PARENTHESES.iter().any(|marker| spelled(fam, each) == marker.0)
    })
}

/// Whether a type annotation under a node of `holder` kind says what it returns.
pub(super) const fn returning(holder: Node) -> Returning {
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
pub(super) const fn kinded(node: Node, field: Field) -> KindRole {
    KindRole { node, field }
}

/// The role of the children after a token spelled `marker`.
pub(super) const fn marked(marker: Marker, field: Field) -> MarkerRole {
    MarkerRole { marker, field }
}

/// The role of a child of `child` kind: the role paired with its kind in `kinds`, else `otherwise`.
pub(super) fn by_kind(child: Kind, kinds: &[KindRole], otherwise: Field) -> Field {
    kinds.iter().find(|each| child == Kind::Node(each.node)).map_or(otherwise, |each| each.field)
}

/// The role of `at` by the tokens before it: the role paired with the first of `markers` a token before it spells, else `otherwise`.
pub(super) fn by_marker(fam: &Family<'_>, at: usize, markers: &[MarkerRole], otherwise: Field) -> Field {
    markers.iter().find(|each| after(fam, at, each.marker) == Precedence::Preceded).map_or(otherwise, |each| each.field)
}
