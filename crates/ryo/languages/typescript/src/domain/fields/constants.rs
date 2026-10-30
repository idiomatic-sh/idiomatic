//! The names of the `fields` concept: the spellings of the tokens that mark where a property of a node begins.

use super::types::Marker;

/// The `=>` before the body of an arrow function.
pub(crate) const ARROW: Marker = Marker("=>");

/// The `as` before the name type of a mapped type.
pub(crate) const AS: Marker = Marker("as");

/// The `)` that closes a parenthesized operand.
pub(crate) const CLOSE_PARENTHESIS: Marker = Marker(")");

/// The `:` that ends the test of a `case`, and before the value of a mapped type.
pub(crate) const COLON: Marker = Marker(":");

/// The `.` before the name an import type reads.
pub(crate) const DOT: Marker = Marker(".");

/// The `=` before the default of a type parameter.
pub(crate) const EQUALS: Marker = Marker("=");

/// The `extends` before a superclass or a type parameter's constraint.
pub(crate) const EXTENDS: Marker = Marker(crate::domain::names::constants::EXTENDS);

/// The `from` before the module an export names.
pub(crate) const FROM: Marker = Marker("from");

/// The `in` before the constraint of a mapped type's key.
pub(crate) const IN: Marker = Marker("in");

/// The `(` before the parameters of a function.
pub(crate) const OPEN_PARENTHESIS: Marker = Marker("(");

/// The `;` that ends a clause of a `for`.
pub(crate) const SEMICOLON: Marker = Marker(";");

/// The parentheses around an operand, which spell no operator.
pub(crate) const PARENTHESES: [Marker; 2] = [OPEN_PARENTHESIS, CLOSE_PARENTHESIS];
