//! The `fields` service: free functions named by verb that say the role of an element under its parent. The tree is TypeScript-ESTree's,
//! and the role of a child is the property of its parent that holds it (`@typescript-eslint/visitor-keys`). The parser's visitor reports
//! no property, so a role is read from the kinds of the parent and the child, from where the child stands among the nodes beside it, and,
//! where a property may be left out, from the token that comes before the child (`(` before parameters, `extends` before a superclass).

use super::holding::service::held;
use super::placing::service::{operator_of, returning};
use super::types::{Field, Returning};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind, kind_bits, kind_name, kinds, parent};
use crate::domain::syntax::types::Kind;
use ryo::languages::{self, Position, Tree};
use strum::IntoEnumIterator as _;

/// The number the element table holds for a role.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a role is a field-less enum with a u16 representation: its number is the cast"
)]
pub(crate) const fn field_bits(field: Field) -> languages::FieldId {
    languages::FieldId(field as u16)
}

/// A role's name, in kebab case: text that leaves the engine.
#[must_use]
pub(crate) fn field_name(field: Field) -> languages::NameText<'static> {
    languages::NameText(field.into())
}

/// Every role, in the order of their numbers.
pub(crate) fn fields() -> impl Iterator<Item = Field> {
    Field::iter()
}

/// The role of the element at `at` under its parent; `None` for the root, for a token that spells no operator, and for an extra.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: Position) -> Field {
    let holder = parent(tree, at);
    if holder == at {
        return Field::None;
    }
    let Kind::Node(node) = kind(tree, holder) else {
        return Field::None;
    };
    let child = kind(tree, at);
    match child {
        Kind::Token(_) if operator_of(tree, holder) == Some(at) => Field::Operator,
        Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => Field::None,
        Kind::Node(Node::Decorator) => Field::Decorators,
        Kind::Node(Node::TSTypeAnnotation) if returning(node) == Returning::Returns => {
            Field::ReturnType
        }
        Kind::Node(Node::TSTypeAnnotation) => Field::TypeAnnotation,
        Kind::Node(Node::TSTypeParameterDeclaration) => Field::TypeParameters,
        Kind::Node(Node::TSTypeParameterInstantiation)
            if matches!(node, Node::ClassDeclaration | Node::ClassExpression) =>
        {
            Field::SuperTypeArguments
        }
        Kind::Node(Node::TSTypeParameterInstantiation) => Field::TypeArguments,
        Kind::Node(_) => held(tree, at, node, child),
    }
}

/// The number of the role of the element at `at`, for the table to hold.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}

/// Gives the completion the names of the language's kinds, and, when the run wants them, of its roles and the role of every element.
pub(crate) fn name_vocabulary(
    completion: &mut languages::Completion<'_>,
    wanted: languages::FieldRequest,
) {
    for kind in kinds() {
        languages::name_kind(completion, kind_bits(kind), kind_name(kind));
    }
    if wanted == languages::FieldRequest::Wanted {
        for field in fields() {
            languages::name_field(completion, field_bits(field), field_name(field));
        }
        languages::write_fields(completion, role_of);
    }
}
