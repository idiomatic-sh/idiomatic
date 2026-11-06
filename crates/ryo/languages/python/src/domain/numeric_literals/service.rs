//! Normalize existing parser-owned payloads; never parse Python source spelling again.

use super::constants::FLOAT_TEXT_BYTES;
use super::types::{NumericClassification, NumericLiteral, NumericValue};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        constants::{COMPLEX_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR, SCALAR_KIND, SCALAR_VALUE},
        service::{kind, literal},
        types::{Kind, Literal},
    },
};
use ryo::languages::{self, ByteSlice, NameText, Position, Symbol, Symbols, Tree};
use std::io::Write as _;
use super::constants;

/// Intern an occurrence without allocating temporary decimal text on each file read.
pub(crate) fn intern_value(
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    numeric: &NumericLiteral<'_>,
) -> Symbol {
    match numeric.value {
        NumericValue::Integer(text) => languages::intern_derived(symbols, source, NameText(text)),
        NumericValue::Float(bits) => {
            let mut buffer = [0; FLOAT_TEXT_BYTES];
            let text = write_float(bits, &mut buffer);
            languages::intern_derived(symbols, source, NameText(text))
        }
    }
}

/// Write a round-trip binary64 spelling into bounded stack storage and borrow its text.
fn write_float(bits: u64, buffer: &mut [u8; FLOAT_TEXT_BYTES]) -> &str {
    let value = f64::from_bits(bits);
    let mut rest: &mut [u8] = &mut buffer[..];
    assert!(
        write!(rest, "{value}").is_ok(),
        "binary64 text fits the fixed buffer"
    );
    let mut written = FLOAT_TEXT_BYTES - rest.len();
    if value.is_finite()
        && !buffer[..written]
            .iter()
            .any(|byte| matches!(byte, b'.' | b'e' | b'E'))
    {
        let mut rest: &mut [u8] = &mut buffer[written..];
        assert!(
            rest.write_all(constants::FLOAT_SUFFIX).is_ok(),
            "binary64 suffix fits the fixed buffer"
        );
        written = FLOAT_TEXT_BYTES - rest.len();
    }
    std::str::from_utf8(&buffer[..written]).expect(constants::ASCII_ONLY)
}

/// The same stack formatter is observed by the payload tests.
#[cfg(test)]
pub(crate) fn float_text(bits: u64) -> String {
    let mut buffer = [0; FLOAT_TEXT_BYTES];
    write_float(bits, &mut buffer).to_owned()
}

/// Validate a parser-normalized integer payload, without imposing a machine-width bound.
fn canonical_integer(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits.len() == 1 || !digits.starts_with('0'))
}

/// Read one primitive occurrence. Unary signs and arithmetic are not constant-folded.
pub(crate) fn classify<'a>(tree: &'a Tree<'_>, at: Position) -> NumericClassification<'a> {
    if kind(tree, at) != Kind::Node(Node::Constant) {
        return NumericClassification::NotNumeric;
    }
    let scalar_kind = languages::property_at(tree.elements, at, SCALAR_KIND).map(|text| text.0);
    let value = languages::property_at(tree.elements, at, SCALAR_VALUE).map(|text| text.0);
    match scalar_kind {
        Some(INTEGER_SCALAR) => match value.filter(|value| canonical_integer(value)) {
            Some(value) => NumericClassification::Known(NumericLiteral {
                kind: crate::domain::syntax::constants::INTEGER_SCALAR,
                value: NumericValue::Integer(value),
            }),
            None => NumericClassification::MissingPayload,
        },
        Some(FLOAT_SCALAR) => match value.and_then(|value| value.parse::<u64>().ok()) {
            Some(bits) => NumericClassification::Known(NumericLiteral {
                kind: crate::domain::syntax::constants::FLOAT_SCALAR,
                value: NumericValue::Float(bits),
            }),
            None => NumericClassification::MissingPayload,
        },
        // The literal discriminator remains available when scalar properties are missing.
        Some(COMPLEX_SCALAR) => NumericClassification::NotNumeric,
        Some(_) | None if literal(tree, at) == Literal::Number => NumericClassification::MissingPayload,
        Some(_) | None => NumericClassification::NotNumeric,
    }
}
