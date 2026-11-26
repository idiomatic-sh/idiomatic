//! The `fields` service: free functions named by verb that say the role of an element under its parent. The tree is TypeScript-ESTree's,
//! and the role of a child is the property of its parent that holds it (`@typescript-eslint/visitor-keys`). The parser's visitor reports
//! no property, so a role is read from the kinds of the parent and the child, from where the child stands among the nodes beside it, and,
//! where a property may be left out, from the token that comes before the child (`(` before parameters, `extends` before a superclass).

use super::types::Field;
use ryo::languages;
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
