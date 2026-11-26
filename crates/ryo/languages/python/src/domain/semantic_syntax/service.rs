//! Select parsed annotation expressions using lexical typing context, never lint rules.
use super::constants::{ANNOTATED, CAST, LITERAL, NEW_TYPE, TYPE_ALIAS, TYPE_VAR, TYPING, TYPING_EXTENSIONS};
use super::types::{TypeContext, TypingForm};

use crate::domain::{nodes::types::{Category}};
use ryo::languages::{self, Position};

/// The typing form a qualified name names: a generic constructor, one of the forms of `typing` that read some of their
/// operands as types, or neither.
#[must_use]
pub(crate) fn typing_form(path: languages::NameText<'_>) -> TypingForm {
    if super::generic_subscription::service::operands(path) == TypeContext::Type {
        return TypingForm::Generic;
    }
    match path
        .0
        .strip_prefix(TYPING)
        .or_else(|| path.0.strip_prefix(TYPING_EXTENSIONS))
    {
        Some(LITERAL) => TypingForm::Literal,
        Some(ANNOTATED) => TypingForm::Annotated,
        Some(CAST) => TypingForm::Cast,
        Some(NEW_TYPE) => TypingForm::NewType,
        Some(TYPE_VAR) => TypingForm::TypeVar,
        Some(TYPE_ALIAS) => TypingForm::TypeAlias,
        Some(_) | None => TypingForm::Other,
    }
}

/// Whether a non-keyword `argument` among the call's `operands` is a type operand of `form`.
/// Reads up to the second operand, or to the first expression for `TypeVar`; never the whole list.
pub(crate) fn operand_place(
    mut operands: impl Iterator<Item = Position>,
    argument: Position,
    form: TypingForm,
    mut category: impl FnMut(Position) -> Category,
) -> TypeContext {
    let is_type = match form {
        TypingForm::Cast => operands.next() == Some(argument),
        TypingForm::NewType => operands.nth(1) == Some(argument),
        TypingForm::TypeVar => {
            category(argument) == Category::Expression
                && operands.find(|at| category(*at) == Category::Expression) != Some(argument)
        }
        TypingForm::Literal
        | TypingForm::Annotated
        | TypingForm::TypeAlias
        | TypingForm::Other
        | TypingForm::Generic => false,
    };
    if is_type {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}
