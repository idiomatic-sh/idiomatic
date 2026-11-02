//! Normalize existing parser-owned payloads; never parse Python source spelling again.

use super::types::{Classification, NumericLiteral, NumericValue};
use crate::domain::{
    nodes::types::Node,
    syntax::{
        constants::{COMPLEX_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR, SCALAR_KIND, SCALAR_VALUE},
        service::{kind, literal},
        types::{Kind, Literal},
    },
};
use ryo::languages::{self, ByteSlice, NameText, Position, Symbol, Symbols, Tree};
use std::fmt::Write as _;

/// `Display`'s longest finite binary64 spelling fits without a heap allocation.
struct FloatTextBuffer {
    bytes: [u8; 350],
    len: usize,
}

impl std::fmt::Write for FloatTextBuffer {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self.len.checked_add(text.len()).ok_or(std::fmt::Error)?;
        let Some(space) = self.bytes.get_mut(self.len..end) else {
            return Err(std::fmt::Error);
        };
        space.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl FloatTextBuffer {
    fn new() -> Self {
        Self {
            bytes: [0; 350],
            len: 0,
        }
    }

    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len])
            .unwrap_or_else(|_| panic!("numeric Display only writes ASCII"))
    }
}

/// Intern an occurrence without allocating temporary decimal text on each file read.
pub(crate) fn intern_value(
    symbols: &mut Symbols,
    source: ByteSlice<'_>,
    numeric: &NumericLiteral<'_>,
) -> Symbol {
    match numeric.value {
        NumericValue::Integer(text) => languages::intern_derived(symbols, source, NameText(text)),
        NumericValue::Float(bits) => {
            let mut buffer = FloatTextBuffer::new();
            write_float(bits, &mut buffer);
            languages::intern_derived(symbols, source, NameText(buffer.text()))
        }
    }
}

/// Write a round-trip binary64 spelling into bounded stack storage.
fn write_float(bits: u64, buffer: &mut FloatTextBuffer) {
    let value = f64::from_bits(bits);
    assert!(
        write!(buffer, "{value}").is_ok(),
        "binary64 text fits the fixed buffer"
    );
    if value.is_finite() && !buffer.text().contains(['.', 'e', 'E']) {
        assert!(
            buffer.write_str(".0").is_ok(),
            "binary64 suffix fits the fixed buffer"
        );
    }
}

/// The same stack formatter is observed by the payload tests.
#[cfg(test)]
pub(crate) fn float_text(bits: u64) -> String {
    let mut buffer = FloatTextBuffer::new();
    write_float(bits, &mut buffer);
    buffer.text().to_owned()
}

/// Validate a parser-normalized integer payload, without imposing a machine-width bound.
fn canonical_integer(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits.len() == 1 || !digits.starts_with('0'))
}

/// Read one primitive occurrence. Unary signs and arithmetic are not constant-folded.
pub(crate) fn classify<'a>(tree: &'a Tree<'_>, at: Position) -> Classification<'a> {
    if kind(tree, at) != Kind::Node(Node::Constant) {
        return Classification::NotNumeric;
    }
    let scalar_kind = languages::property_at(tree.elements, at, SCALAR_KIND).map(|text| text.0);
    let value = languages::property_at(tree.elements, at, SCALAR_VALUE).map(|text| text.0);
    match scalar_kind {
        Some(INTEGER_SCALAR) => match value.filter(|value| canonical_integer(value)) {
            Some(value) => Classification::Known(NumericLiteral {
                kind: "integer",
                value: NumericValue::Integer(value),
            }),
            None => Classification::MissingPayload,
        },
        Some(FLOAT_SCALAR) => match value.and_then(|value| value.parse::<u64>().ok()) {
            Some(bits) => Classification::Known(NumericLiteral {
                kind: "float",
                value: NumericValue::Float(bits),
            }),
            None => Classification::MissingPayload,
        },
        // The literal discriminator remains available when scalar properties are missing.
        Some(COMPLEX_SCALAR) => Classification::NotNumeric,
        Some(_) | None if literal(tree, at) == Literal::Number => Classification::MissingPayload,
        Some(_) | None => Classification::NotNumeric,
    }
}
