//! Normalize existing parser-owned payloads; never parse Python source spelling again.

use super::types::{NumericClassification, NumericLiteral, NumericValue};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        constants::{COMPLEX_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR, SCALAR_KIND, SCALAR_VALUE},
        service::{kind, literal},
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
