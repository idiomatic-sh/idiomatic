//! Normalize parser-provided numeric literals into exact, hashable constant-key numbers.
use crate::domain::{
    constant_numbers::types::{Binary64, ExactInteger, FloatPayload, NumberKey, RealKey},
    syntax::{
        constants::{
            BOOLEAN_SCALAR, COMPLEX_SCALAR, FLOAT_SCALAR, INTEGER_SCALAR, SCALAR_IMAGINARY,
            SCALAR_KIND, SCALAR_VALUE,
        },
        service::literal,
    },
};
use ryo::languages::{self, Position, Tree};

/// Read one parser-owned scalar property from an element.
pub(crate) fn property<'a>(
    tree: &'a Tree<'_>,
    at: Position,
    id: languages::PropertyId,
) -> Option<languages::NameText<'a>> {
    languages::property_at(tree.elements, at, id).map(|value| languages::NameText(value.0))
}

/// The first binary64 value outside the nonnegative `u64` normalization range.
const U64_LIMIT: Binary64 = Binary64(f64::from_bits(0x43f0_0000_0000_0000));

/// Normalize a float to an integer only when a bounded conversion round-trips exactly.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "the upper bound and exact round trip prove this value is an exact supported integer"
)]
pub(crate) fn float_key(value: Binary64) -> RealKey {
    let value = value.0;
    if value == 0.0_f64 {
        return RealKey::Integer(ExactInteger(0));
    }
    if value.is_finite() && value.abs() < U64_LIMIT.0 {
        let integer = value as i128;
        if integer as f64 == value {
            return RealKey::Integer(ExactInteger(integer));
        }
    }
    RealKey::Float(FloatPayload(value.to_bits()))
}

/// Normalize a parser-provided numeric or boolean literal.
pub(crate) fn number(tree: &Tree<'_>, at: Position) -> Option<NumberKey> {
    let scalar = property(tree, at, SCALAR_KIND)?.0;
    let zero = RealKey::Integer(ExactInteger(0));
    let answer = match scalar {
        INTEGER_SCALAR => match property(tree, at, SCALAR_VALUE)?.0.parse::<u64>() {
            Ok(value) => NumberKey {
                real: RealKey::Integer(ExactInteger(i128::from(value))),
                imaginary: zero,
            },
            Err(_) => return None,
        },
        FLOAT_SCALAR => {
            let Ok(bits) = property(tree, at, SCALAR_VALUE)?.0.parse() else {
                return None;
            };
            NumberKey {
                real: float_key(Binary64(f64::from_bits(bits))),
                imaginary: zero,
            }
        }
        COMPLEX_SCALAR => {
            let Ok(real) = property(tree, at, SCALAR_VALUE)?.0.parse() else {
                return None;
            };
            let Ok(imaginary) = property(tree, at, SCALAR_IMAGINARY)?.0.parse() else {
                return None;
            };
            NumberKey {
                real: float_key(Binary64(f64::from_bits(real))),
                imaginary: float_key(Binary64(f64::from_bits(imaginary))),
            }
        }
        BOOLEAN_SCALAR => {
            let Ok(truth) = property(tree, at, SCALAR_VALUE)?.0.parse::<bool>() else {
                return None;
            };
            NumberKey {
                real: RealKey::Integer(ExactInteger(i128::from(u8::from(truth)))),
                imaginary: zero,
            }
        }
        _ => return None,
    };
    Some(answer)
}

/// Negate one exact integer or IEEE float key.
pub(crate) fn negate(value: RealKey) -> Option<RealKey> {
    match value {
        RealKey::Integer(value) => value
            .0
            .checked_neg()
            .map(|negated| RealKey::Integer(ExactInteger(negated))),
        RealKey::Float(bits) => Some(RealKey::Float(FloatPayload(
            (-f64::from_bits(bits.0)).to_bits(),
        ))),
    }
}
/// Convert a real key back to its floating value for complex literal composition.
pub(crate) fn to_float(value: RealKey) -> Binary64 {
    match value {
        RealKey::Integer(value) => integer_as_float(value),
        RealKey::Float(bits) => Binary64(f64::from_bits(bits.0)),
    }
}
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "Python complex arithmetic converts the real operand to f64 before composing the result"
)]
/// Match Python complex arithmetic's conversion of a real operand to binary64.
fn integer_as_float(value: ExactInteger) -> Binary64 {
    Binary64(value.0 as f64)
}
