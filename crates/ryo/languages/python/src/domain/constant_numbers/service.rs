//! Normalize parser-provided numeric literals into exact, hashable constant-key numbers.
use crate::domain::{constant_numbers::types::{Binary64, ExactInteger, FloatPayload, RealKey}};


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
