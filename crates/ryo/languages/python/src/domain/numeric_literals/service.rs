//! Normalize existing parser-owned payloads; never parse Python source spelling again.

use super::types::{NumericClassification, NumericLiteral, NumericValue};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        constants::{
            COMPLEX_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR, SCALAR_KIND, SCALAR_VALUE, WRITTEN_DIGITS_BYTES,
        },
        service::{kind, literal, written},
        types::{Kind, Literal},
    },
};
use ryo::languages::{
    self, ByteSlice, FLOAT_TEXT_BYTES, Flag, FloatBits, FloatBuffer, NameText, Position, Symbol,
    Symbols, Tree,
};

/// Intern an occurrence without allocating temporary decimal text on each file read.
pub(crate) fn intern_value(
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    numeric: &NumericLiteral<'_>,
) -> Symbol {
    match numeric.value {
        NumericValue::Integer(text) => languages::intern_derived(symbols, source, text),
        NumericValue::Float(bits) => {
            let mut buffer = FloatBuffer([0; FLOAT_TEXT_BYTES]);
            let text = languages::binary64_text(bits, &mut buffer);
            languages::intern_derived(symbols, source, text)
        }
    }
}

/// Whether the literal at `at` writes digits its value gives back, the `exact-digits` cell: an integer is exact at any size, and
/// a float's digits are its decimal spelling before the exponent, without `_` and leading zeros, copied to the stack. Digits
/// that do not fit [`WRITTEN_DIGITS_BYTES`] are more than binary64 can hold, so they are not exact.
#[must_use]
pub(crate) fn exactness(tree: &Tree<'_>, at: Position, numeric: &NumericLiteral<'_>) -> Flag {
    let NumericValue::Float(bits) = numeric.value else {
        return Flag::Raised;
    };
    let mut digits = [0_u8; WRITTEN_DIGITS_BYTES];
    let mut length = 0;
    for byte in written(tree, at)
        .0
        .bytes()
        .take_while(|byte| !matches!(byte, b'e' | b'E'))
        .filter(|byte| *byte != b'_')
        .skip_while(|byte| *byte == b'0')
    {
        let Some(slot) = digits.get_mut(length) else {
            return Flag::Lowered;
        };
        *slot = byte;
        length += 1;
    }
    match digits.get(..length).map(std::str::from_utf8) {
        Some(Ok(text)) => languages::exact_digits(NameText(text), bits),
        // digits copied from a float's spelling are ASCII; a cut that is not text writes no digits to give back
        Some(Err(_)) | None => Flag::Lowered,
    }
}

/// Validate a parser-normalized integer payload, without imposing a machine-width bound.
fn canonical_integer(value: NameText<'_>) -> Flag {
    let digits = value.0.strip_prefix('-').unwrap_or(value.0);
    if !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits.len() == 1 || !digits.starts_with('0'))
    {
        Flag::Raised
    } else {
        Flag::Lowered
    }
}

/// Read one primitive occurrence. Unary signs and arithmetic are not constant-folded.
pub(crate) fn classify<'a>(tree: &'a Tree<'_>, at: Position) -> NumericClassification<'a> {
    if kind(tree, at) != Kind::Node(Node::Constant) {
        return NumericClassification::NotNumeric;
    }
    let scalar_kind = languages::property_at(tree.elements, at, SCALAR_KIND);
    let value = languages::property_at(tree.elements, at, SCALAR_VALUE);
    match scalar_kind.map(|each| each.0) {
        Some(INTEGER_SCALAR) => {
            match value.filter(|value| canonical_integer(*value) == Flag::Raised) {
                Some(value) => NumericClassification::Known(NumericLiteral {
                    kind: NameText(crate::domain::syntax::constants::INTEGER_SCALAR),
                    value: NumericValue::Integer(value),
                }),
                None => NumericClassification::MissingPayload,
            }
        }
        Some(FLOAT_SCALAR) => match value.and_then(|value| match value.0.parse::<u64>() {
            Ok(parsed) => Some(parsed),
            Err(_) => None,
        }) {
            Some(bits) => NumericClassification::Known(NumericLiteral {
                kind: NameText(crate::domain::syntax::constants::FLOAT_SCALAR),
                value: NumericValue::Float(FloatBits(bits)),
            }),
            None => NumericClassification::MissingPayload,
        },
        // The literal discriminator remains available when scalar properties are missing.
        Some(COMPLEX_SCALAR) => NumericClassification::NotNumeric,
        Some(_) | None if literal(tree, at) == Literal::Number => {
            NumericClassification::MissingPayload
        }
        Some(_) | None => NumericClassification::NotNumeric,
    }
}
