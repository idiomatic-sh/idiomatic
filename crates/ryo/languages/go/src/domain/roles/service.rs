//! The `roles` service: the role of an element under its parent, the field of `go/ast` that holds it, read from the kinds of the
//! two and from where the element stands among its siblings; a token's is the `token.Pos` field that says where it is. A function's
//! name and receiver, which `go/ast` holds in `FuncDecl`, are under its `FuncType` (which starts at `func`) and keep those roles.

use ryo::languages::{self};
use strum::IntoEnumIterator as _;

use super::types::AstField;







/// The number the element table holds for a role.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a role is a field-less enum with a u16 representation: its number is the cast"
)]
pub(crate) const fn field_bits(field: AstField) -> languages::FieldId {
    languages::FieldId(field as u16)
}

/// A role's name, in kebab case: text that leaves the engine.
#[must_use]
pub(crate) fn field_name(field: AstField) -> languages::NameText<'static> {
    languages::NameText(field.into())
}

/// Every role, in the order of their numbers.
pub(crate) fn fields() -> impl Iterator<Item = AstField> {
    AstField::iter()
}
