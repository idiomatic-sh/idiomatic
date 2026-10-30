//! The names of the `fields` concept: the spellings of the tokens that mark where a property of a node begins.

use super::types::Marker;

/// The `=>` before the body of an arrow function.
pub(crate) const ARROW: Marker = Marker(crate::domain::syntax::constants::ARROW);

/// The `)` that closes a parenthesized operand.
pub(crate) const CLOSE_PARENTHESIS: Marker = Marker(")");

/// The `:` that ends the test of a `case`.
pub(crate) const COLON: Marker = Marker(":");

/// The `extends` before a superclass.
pub(crate) const EXTENDS: Marker = Marker(crate::domain::names::constants::EXTENDS);

/// The `from` before the module an export names.
pub(crate) const FROM: Marker = Marker("from");

/// The `(` before the parameters of a function.
pub(crate) const OPEN_PARENTHESIS: Marker = Marker("(");

/// The `;` that ends a clause of a `for`.
pub(crate) const SEMICOLON: Marker = Marker(";");

/// The parentheses around an operand, which spell no operator.
pub(crate) const PARENTHESES: [Marker; 2] = [OPEN_PARENTHESIS, CLOSE_PARENTHESIS];
