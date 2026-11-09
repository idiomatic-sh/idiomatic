//! Read normalized numeric text and compare without rounding integers through binary64.

use super::constants::{INFINITY, NEGATIVE_INFINITY, NOT_A_NUMBER};
use super::types::{
    Decimal, DecimalDigits, FloatForm, FloatValue, Magnitude, Negative, NumberText, Numeric, Shift,
    SmallInteger, Succession,
};
use num_bigint::{BigInt, BigUint};
use ryo_values::domain::plan_parts::types::WholeNumber;
use std::cmp::Ordering;

/// Read canonical signed decimal integer text. Spellings of any source language and whitespace are not accepted.
fn decimal(text: NumberText<'_>) -> Option<Decimal<'_>> {
    let (negative, digits) = text
        .0
        .strip_prefix('-')
        .map_or((false, text.0), |digits| (true, digits));
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
    {
        return None;
    }
    let nonzero = digits.bytes().any(|byte| byte != b'0');
    Some(Decimal {
        negative: Negative(negative && nonzero),
        digits: DecimalDigits(digits),
    })
}

/// Whether text has a decimal binary64 spelling with a dot or exponent marker.
fn float_spelling(text: NumberText<'_>) -> FloatForm {
    let bytes = text.0.strip_prefix('-').unwrap_or(text.0).as_bytes();
    let mut at = 0;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    if at == 0 || (at > 1 && bytes.first() == Some(&b'0')) {
        return FloatForm::Malformed;
    }
    let mut marked = false;
    if bytes.get(at) == Some(&b'.') {
        marked = true;
        at += 1;
        let begin = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == begin {
            return FloatForm::Malformed;
        }
    }
    if bytes
        .get(at)
        .is_some_and(|byte| matches!(*byte, b'e' | b'E'))
    {
        marked = true;
        at += 1;
        if bytes
            .get(at)
            .is_some_and(|byte| matches!(*byte, b'+' | b'-'))
        {
            at += 1;
        }
        let begin = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == begin {
            return FloatForm::Malformed;
        }
    }
    if marked && at == bytes.len() {
        FloatForm::Spelled
    } else {
        FloatForm::Malformed
    }
}

/// Read decimal integers, marked decimal floats, `inf`, `-inf`, and `NaN`.
/// No source-language radix, underscore, coercion, or whitespace normalization occurs here.
pub(crate) fn read(text: NumberText<'_>) -> Option<Numeric<'_>> {
    match text.0 {
        INFINITY => Some(Numeric::Float(FloatValue(f64::INFINITY))),
        NEGATIVE_INFINITY => Some(Numeric::Float(FloatValue(f64::NEG_INFINITY))),
        NOT_A_NUMBER => Some(Numeric::Float(FloatValue(f64::NAN))),
        _ if text.0.contains(['.', 'e', 'E']) => {
            if float_spelling(text) == FloatForm::Malformed {
                return None;
            }
            match text.0.parse::<f64>() {
                Ok(number) => Some(Numeric::Float(FloatValue(number))),
                Err(_) => None,
            }
        }
        _ => decimal(text).map(Numeric::Decimal),
    }
}

/// Compare normalized decimal magnitudes without allocation.
fn decimal_magnitude(left: DecimalDigits<'_>, right: DecimalDigits<'_>) -> Ordering {
    left.0
        .len()
        .cmp(&right.0.len())
        .then_with(|| left.0.cmp(right.0))
}

/// Compare an arbitrary decimal integer to an existing signed number.
fn decimal_whole(left: Decimal<'_>, right: WholeNumber) -> Ordering {
    let right = right.0;
    let right_negative = Negative(right < 0);
    if left.negative != right_negative {
        return if left.negative.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    let order = left
        .digits
        .0
        .parse::<u64>()
        .map_or(Ordering::Greater, |left| left.cmp(&right.unsigned_abs()));
    if left.negative.0 {
        order.reverse()
    } else {
        order
    }
}

/// Scale a small magnitude only when the complete exact product fits.
fn scaled(value: Magnitude, shift: Shift) -> Option<Magnitude> {
    if value.0 == 0 {
        return Some(Magnitude(0));
    }
    value
        .0
        .checked_mul(1_u128.checked_shl(shift.0)?)
        .map(Magnitude)
}

/// Compare one nonnegative integer magnitude with one finite nonnegative float.
/// FloatValue is an integer significand times a power of two; no integer-to-float cast is used.
fn magnitude_float(
    small: Option<Magnitude>,
    digits: Option<DecimalDigits<'_>>,
    float: FloatValue,
) -> Option<Ordering> {
    let bits = float.0.to_bits();
    let exponent_bits = u16::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if exponent_bits == 0 {
        (fraction, -1074_i32)
    } else {
        (fraction | (1_u64 << 52), i32::from(exponent_bits) - 1075)
    };
    if let Some(integer) = small {
        return Some(if exponent >= 0 {
            scaled(
                Magnitude(u128::from(significand)),
                Shift(exponent.unsigned_abs()),
            )
            .map_or(Ordering::Less, |right| integer.0.cmp(&right.0))
        } else {
            scaled(integer, Shift(exponent.unsigned_abs())).map_or(Ordering::Greater, |left| {
                left.0.cmp(&u128::from(significand))
            })
        });
    }
    // A decimal magnitude wider than u128 exceeds every float with a nonpositive scale.
    if exponent <= 0 {
        return Some(Ordering::Greater);
    }
    // Only the genuinely wide mixed case allocates; use the already pinned big-integer crate.
    let integer = BigUint::parse_bytes(digits?.0.as_bytes(), 10)?;
    let right = BigUint::from(significand) << exponent.unsigned_abs();
    Some(integer.cmp(&right))
}

/// Compare an integer operand to a binary64 operand, retaining signed-zero and NaN semantics.
fn integer_float(integer: Numeric<'_>, float: FloatValue) -> Option<Ordering> {
    let float = float.0;
    if float.is_nan() {
        return None;
    }
    if float == f64::INFINITY {
        return Some(Ordering::Less);
    }
    if float == f64::NEG_INFINITY {
        return Some(Ordering::Greater);
    }
    let (negative, small, digits) = match integer {
        Numeric::Decimal(integer) => (
            integer.negative.0,
            match integer.digits.0.parse::<u128>() {
                Ok(magnitude) => Some(Magnitude(magnitude)),
                Err(_) => None,
            },
            Some(integer.digits),
        ),
        Numeric::Whole(integer) => (
            integer.0 < 0,
            Some(Magnitude(u128::from(integer.0.unsigned_abs()))),
            None,
        ),
        Numeric::Float(_) => return None,
    };
    let float_negative = float < 0.0;
    if negative != float_negative {
        return Some(if negative {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let order = magnitude_float(small, digits, FloatValue(float.abs()))?;
    Some(if negative { order.reverse() } else { order })
}

/// Exact numeric ordering. NaN is unordered, including against itself.
pub(crate) fn compare(left: Numeric<'_>, right: Numeric<'_>) -> Option<Ordering> {
    match (left, right) {
        (Numeric::Decimal(left), Numeric::Decimal(right)) => {
            Some(left.negative.cmp(&right.negative).reverse().then_with(|| {
                let order = decimal_magnitude(left.digits, right.digits);
                if left.negative.0 {
                    order.reverse()
                } else {
                    order
                }
            }))
        }
        (Numeric::Whole(left), Numeric::Whole(right)) => Some(left.cmp(&right)),
        (Numeric::Decimal(left), Numeric::Whole(right)) => Some(decimal_whole(left, right)),
        (Numeric::Whole(left), Numeric::Decimal(right)) => {
            Some(decimal_whole(right, left).reverse())
        }
        (Numeric::Float(left), Numeric::Float(right)) => left.0.partial_cmp(&right.0),
        (integer @ (Numeric::Decimal(_) | Numeric::Whole(_)), Numeric::Float(float)) => {
            integer_float(integer, float)
        }
        (Numeric::Float(float), integer @ (Numeric::Decimal(_) | Numeric::Whole(_))) => {
            integer_float(integer, float).map(Ordering::reverse)
        }
    }
}

/// Read a narrow exact integer without allocating; floats have no integer coercion here.
fn small_integer(value: Numeric<'_>) -> Option<SmallInteger> {
    match value {
        Numeric::Whole(value) => Some(SmallInteger(i128::from(value.0))),
        Numeric::Decimal(value) => {
            let Ok(magnitude) = value.digits.0.parse::<i128>() else {
                return None;
            };
            if value.negative.0 {
                magnitude.checked_neg().map(SmallInteger)
            } else {
                Some(SmallInteger(magnitude))
            }
        }
        Numeric::Float(_) => None,
    }
}

/// Read only integer variants through the pinned arbitrary precision implementation.
fn wide_integer(value: Numeric<'_>) -> Option<BigInt> {
    match value {
        Numeric::Whole(value) => Some(BigInt::from(value.0)),
        Numeric::Decimal(value) => {
            let magnitude = BigInt::parse_bytes(value.digits.0.as_bytes(), 10)?;
            Some(if value.negative.0 {
                -magnitude
            } else {
                magnitude
            })
        }
        Numeric::Float(_) => None,
    }
}

/// Whether `right` is exactly `left + 1`, accepting integers without any float coercion.
pub(crate) fn integer_successor(left: Numeric<'_>, right: Numeric<'_>) -> Succession {
    if matches!(left, Numeric::Float(_)) || matches!(right, Numeric::Float(_)) {
        return Succession::NotSuccessor;
    }
    let successor = if let (Some(left), Some(right)) = (small_integer(left), small_integer(right)) {
        left.0.checked_add(1) == Some(right.0)
    } else {
        match (wide_integer(left), wide_integer(right)) {
            (Some(left), Some(right)) => left + BigInt::from(1_u8) == right,
            _ => false,
        }
    };
    if successor {
        Succession::Successor
    } else {
        Succession::NotSuccessor
    }
}
