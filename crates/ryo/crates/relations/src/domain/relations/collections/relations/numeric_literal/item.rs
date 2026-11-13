//! Canonical numeric payloads retain the original occurrence and source interval.

use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// The optional `numeric-literal` relation.
#[derive(Clone, Copy, Debug)]
pub struct NumericLiteral;

impl super::super::interface::sealed::Sealed for NumericLiteral {}

/// Declare parser-decoded numeric literals without constraining integer precision to row words.
#[macro_export]
#[doc(hidden)]
macro_rules! numeric_literal {
    ($callback:path) => {
        $callback! {
            item NumericLiteral,
            row {
                /// One row of `numeric-literal`.
                NumericLiteralRow
            },
            column {
                /// The columns of `numeric-literal`.
                NumericLiteralColumn
            },
            expected {
                /// One expected parser-decoded literal occurrence.
                NumericLiteralExpectedRow
            },
            class Code,
            columns {
                /// Original numeric literal occurrence; unary operators remain separate syntax.
                id Id: NodeId => Node;
                /// `integer` or `float`; booleans and complex numbers are excluded.
                kind Kind: Symbol => Symbol;
                /// Canonical decimal integer or round-trip binary64 text. Floats keep a dot or
                /// exponent marker, except the explicit `inf`, `-inf`, and `NaN` spellings.
                value Value: Symbol => Symbol;
                /// Whether the value printed to as many significant digits as the literal writes gives the written digits
                /// back, in the base the literal is written in; an integer of any size always does.
                exact_digits ExactDigits: Flag => Flag;
                /// File containing the occurrence.
                file File: FileId => File;
                /// Original numeric literal source interval.
                span Span: Span => Span;
            }
        }
    };
}

crate::numeric_literal!(crate::relation_row);
