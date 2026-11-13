//! Normalize existing parser-owned payloads; never parse Python source spelling again.

use super::constants;
use super::constants::FLOAT_TEXT_BYTES;
use super::types::{FloatBits, FloatBuffer, NumericClassification, NumericLiteral, NumericValue};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        constants::{COMPLEX_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR, SCALAR_KIND, SCALAR_VALUE},
        service::{kind, literal},
        types::{Kind, Literal},
    },
};
use ryo::languages::{self, ByteSlice, Flag, NameText, Position, Symbol, Symbols, Tree};

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
            let text = write_float(bits, &mut buffer);
            languages::intern_derived(symbols, source, text)
        }
    }
}

/// Write a round-trip binary64 spelling into bounded stack storage and borrow its text.
#[expect(
    clippy::expect_used,
    reason = "f64's Display and the `.0` suffix write only ASCII, so the written bytes are always UTF-8"
)]
pub(super) fn write_float(bits: FloatBits, buffer: &mut FloatBuffer) -> NameText<'_> {
    let value = f64::from_bits(bits.0);
    let mut rest: &mut [u8] = &mut buffer.0[..];
    assert!(
        std::io::Write::write_fmt(&mut rest, format_args!("{value}")).is_ok(),
        "binary64 text fits the fixed buffer"
    );
    let mut written = FLOAT_TEXT_BYTES - rest.len();
    if value.is_finite()
        && !buffer.0[..written]
            .iter()
            .any(|byte| matches!(byte, b'.' | b'e' | b'E'))
    {
        let mut rest: &mut [u8] = &mut buffer.0[written..];
        assert!(
            std::io::Write::write_all(&mut rest, constants::FLOAT_SUFFIX).is_ok(),
            "binary64 suffix fits the fixed buffer"
        );
        written = FLOAT_TEXT_BYTES - rest.len();
    }
    NameText(std::str::from_utf8(&buffer.0[..written]).expect(constants::ASCII_ONLY))
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
