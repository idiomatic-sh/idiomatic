//! Read normalized numeric text and compare without rounding integers through binary64.

use super::types::{Decimal, Numeric};
use num_bigint::{BigInt, BigUint};
use std::cmp::Ordering;

/// Read canonical signed decimal integer text. Python spelling and whitespace are not accepted.
fn decimal(text: &str) -> Option<Decimal<'_>> {
    let (negative, digits) = text
        .strip_prefix('-')
        .map_or((false, text), |digits| (true, digits));
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
    {
        return None;
    }
    Some(Decimal {
        negative: negative && digits != "0",
        digits,
    })
}

/// Whether text has a decimal binary64 spelling with a dot or exponent marker.
fn float_spelling(text: &str) -> bool {
    let bytes = text.strip_prefix('-').unwrap_or(text).as_bytes();
    let mut at = 0;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    if at == 0 || (at > 1 && bytes.first() == Some(&b'0')) {
        return false;
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
            return false;
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
            return false;
        }
    }
    marked && at == bytes.len()
}

/// Read decimal integers, marked decimal floats, `inf`, `-inf`, and `NaN`.
/// No source-language radix, underscore, coercion, or whitespace normalization occurs here.
pub(crate) fn read(text: &str) -> Option<Numeric<'_>> {
    match text {
        "inf" => Some(Numeric::Float(f64::INFINITY)),
        "-inf" => Some(Numeric::Float(f64::NEG_INFINITY)),
        "NaN" => Some(Numeric::Float(f64::NAN)),
        _ if text.contains(['.', 'e', 'E']) => float_spelling(text)
            .then(|| text.parse::<f64>().ok())
            .flatten()
            .map(Numeric::Float),
        _ => decimal(text).map(Numeric::Decimal),
    }
}

/// Compare normalized decimal magnitudes without allocation.
fn decimal_magnitude(left: &str, right: &str) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

/// Compare an arbitrary decimal integer to an existing signed number.
fn decimal_whole(left: Decimal<'_>, right: i64) -> Ordering {
    let right_negative = right < 0;
    if left.negative != right_negative {
        return if left.negative {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    let order = left
        .digits
        .parse::<u64>()
        .map_or(Ordering::Greater, |left| left.cmp(&right.unsigned_abs()));
    if left.negative {
        order.reverse()
    } else {
        order
    }
}

/// Scale a small magnitude only when the complete exact product fits.
fn scaled(value: u128, shift: u32) -> Option<u128> {
    if value == 0 {
        return Some(0);
    }
    value.checked_mul(1_u128.checked_shl(shift)?)
}

/// Compare one nonnegative integer magnitude with one finite nonnegative float.
/// Binary64 is an integer significand times a power of two; no integer-to-float cast is used.
fn magnitude_float(small: Option<u128>, digits: Option<&str>, float: f64) -> Option<Ordering> {
    let bits = float.to_bits();
    let exponent_bits = u16::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if exponent_bits == 0 {
        (fraction, -1074_i32)
    } else {
        (fraction | (1_u64 << 52), i32::from(exponent_bits) - 1075)
    };
    if let Some(integer) = small {
        return Some(if exponent >= 0 {
            scaled(u128::from(significand), exponent.unsigned_abs())
                .map_or(Ordering::Less, |right| integer.cmp(&right))
        } else {
            scaled(integer, exponent.unsigned_abs())
                .map_or(Ordering::Greater, |left| left.cmp(&u128::from(significand)))
        });
    }
    // A decimal magnitude wider than u128 exceeds every float with a nonpositive scale.
    if exponent <= 0 {
        return Some(Ordering::Greater);
    }
    // Only the genuinely wide mixed case allocates; use the already pinned big-integer crate.
    let integer = BigUint::parse_bytes(digits?.as_bytes(), 10)?;
    let right = BigUint::from(significand) << exponent.unsigned_abs();
    Some(integer.cmp(&right))
}

/// Compare an integer operand to a binary64 operand, retaining signed-zero and NaN semantics.
fn integer_float(integer: Numeric<'_>, float: f64) -> Option<Ordering> {
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
            integer.negative,
            integer.digits.parse::<u128>().ok(),
            Some(integer.digits),
        ),
        Numeric::Whole(integer) => (integer < 0, Some(u128::from(integer.unsigned_abs())), None),
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
    let order = magnitude_float(small, digits, float.abs())?;
    Some(if negative { order.reverse() } else { order })
}

/// Exact numeric ordering. NaN is unordered, including against itself.
pub(crate) fn compare(left: Numeric<'_>, right: Numeric<'_>) -> Option<Ordering> {
    match (left, right) {
        (Numeric::Decimal(left), Numeric::Decimal(right)) => {
            Some(left.negative.cmp(&right.negative).reverse().then_with(|| {
                let order = decimal_magnitude(left.digits, right.digits);
                if left.negative {
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
        (Numeric::Float(left), Numeric::Float(right)) => left.partial_cmp(&right),
        (integer @ (Numeric::Decimal(_) | Numeric::Whole(_)), Numeric::Float(float)) => {
            integer_float(integer, float)
        }
        (Numeric::Float(float), integer @ (Numeric::Decimal(_) | Numeric::Whole(_))) => {
            integer_float(integer, float).map(Ordering::reverse)
        }
    }
}

/// Read a narrow exact integer without allocating; floats have no integer coercion here.
fn small_integer(value: Numeric<'_>) -> Option<i128> {
    match value {
        Numeric::Whole(value) => Some(i128::from(value)),
        Numeric::Decimal(value) => {
            let magnitude = value.digits.parse::<i128>().ok()?;
            if value.negative {
                magnitude.checked_neg()
            } else {
                Some(magnitude)
            }
        }
        Numeric::Float(_) => None,
    }
}

/// Read only integer variants through the pinned arbitrary precision implementation.
fn wide_integer(value: Numeric<'_>) -> Option<BigInt> {
    match value {
        Numeric::Whole(value) => Some(BigInt::from(value)),
        Numeric::Decimal(value) => {
            let magnitude = BigInt::parse_bytes(value.digits.as_bytes(), 10)?;
            Some(if value.negative {
                -magnitude
            } else {
                magnitude
            })
        }
        Numeric::Float(_) => None,
    }
}

/// Whether `right` is exactly `left + 1`, accepting integers without any float coercion.
pub(crate) fn integer_successor(left: Numeric<'_>, right: Numeric<'_>) -> bool {
    if matches!(left, Numeric::Float(_)) || matches!(right, Numeric::Float(_)) {
        return false;
    }
    if let (Some(left), Some(right)) = (small_integer(left), small_integer(right)) {
        return left.checked_add(1) == Some(right);
    }
    match (wide_integer(left), wide_integer(right)) {
        (Some(left), Some(right)) => left + BigInt::from(1_u8) == right,
        _ => false,
    }
}
