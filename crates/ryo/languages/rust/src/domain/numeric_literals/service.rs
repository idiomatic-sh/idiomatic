//! The `numeric_literals` service: free functions named by verb that read what a literal node holds of a number, with no IO and no
//! ports.

use std::borrow::Cow;

use super::constants::{
    BASES, F32_SUFFIX, F64_SUFFIX, FLOAT_NUMBER, INTEGER_SUFFIXES, NUMERIC_KIND, NUMERIC_VALUE,
};
use super::types::{DecodedNumber, IntegerValue, LiteralNumber, NumberValue};
use crate::domain::syntax::service::{child, end, start};
use crate::domain::syntax::types::{Kind, TokenText};
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Flag, FloatBits, NameText, Position, Tree, WRITTEN_DIGITS_BYTES};

/// What the literal node at `at` holds of a number: its kind and value as the adapter stored them, that it is a number with none, or
/// that it is no number.
pub(crate) fn stored<'a>(tree: &Tree<'a>, at: Position) -> LiteralNumber<'a> {
    let number = [Token::IntNumber, Token::FloatNumber]
        .into_iter()
        .any(|token| child(tree, at, Kind::Token(token)).is_some());
    if !number {
        return LiteralNumber::NotNumeric;
    }
    match (
        languages::property_at(tree.elements, at, NUMERIC_KIND),
        languages::property_at(tree.elements, at, NUMERIC_VALUE),
    ) {
        (Some(kind), Some(value)) => LiteralNumber::Known {
            kind,
            value,
            exact: exactness(tree, at, kind, value),
        },
        (Some(_) | None, _) => LiteralNumber::Unread,
    }
}

/// Whether the literal at `at`, of `kind` and `value` as the adapter stored them, writes digits its value gives back: an integer
/// is exact at any size, and a float's digits are its decimal spelling before the exponent and the suffix, without `_` and
/// leading zeros, copied to the stack. Digits that do not fit [`WRITTEN_DIGITS_BYTES`] are more than binary64 can hold, so they
/// are not exact.
fn exactness(tree: &Tree<'_>, at: Position, kind: NameText<'_>, value: NameText<'_>) -> Flag {
    if kind.0 != FLOAT_NUMBER {
        return Flag::Raised;
    }
    let Ok(bits) = value
        .0
        .parse::<f64>()
        .map(|parsed| FloatBits(parsed.to_bits()))
    else {
        // a float without its decoded value has no digits to give back
        return Flag::Lowered;
    };
    let written = tree
        .source
        .0
        .get(
            usize::try_from(start(tree, at).0).unwrap_or(usize::MAX)
                ..usize::try_from(end(tree, at).0).unwrap_or(usize::MAX),
        )
        .unwrap_or_default();
    let mut digits = [0_u8; WRITTEN_DIGITS_BYTES];
    let mut length = 0;
    for byte in written
        .iter()
        .take_while(|byte| byte.is_ascii_digit() || matches!(byte, b'_' | b'.'))
        .filter(|byte| **byte != b'_')
        .skip_while(|byte| **byte == b'0')
    {
        let Some(slot) = digits.get_mut(length) else {
            return Flag::Lowered;
        };
        *slot = *byte;
        length += 1;
    }
    match digits.get(..length).map(std::str::from_utf8) {
        Some(Ok(text)) => languages::exact_digits(NameText(text), bits),
        // digits copied from a float's spelling are ASCII; a cut that is not text writes no digits to give back
        Some(Err(_)) | None => Flag::Lowered,
    }
}

/// What the numeric literal written as `written` stands for, as the lexer gave its token: digits with a base prefix or none, `_`
/// between them, a fraction and an exponent for a float, then a type suffix (the Rust Reference, Integer literals and Floating-point
/// literals). None when a compiler refuses it: a suffix that names no numeric type of the literal's base (`1.0u8`, `0b1f32`, `1f16`),
/// or digits with no value (one outside the base, an integer past 128 bits).
pub(crate) fn decoded(written: TokenText<'_>) -> Option<DecodedNumber<'_>> {
    let text = written.0;
    let (suffix, value) = if let Some((rest, radix)) = BASES
        .iter()
        .find_map(|(prefix, radix)| Some((text.strip_prefix(prefix)?, *radix)))
    {
        let end = rest
            .find(|character: char| character != '_' && !character.is_digit(radix.max(10)))
            .unwrap_or(rest.len());
        let (digits, suffix) = rest.split_at(end);
        if !suffix.is_empty() && !INTEGER_SUFFIXES.contains(&suffix) {
            return None;
        }
        (suffix, NumberValue::Integer(integer(digits, radix)?))
    } else {
        let (end, floating) = decimal_end(text);
        let (digits, suffix) = text.split_at(end);
        let value = if !floating && (suffix.is_empty() || INTEGER_SUFFIXES.contains(&suffix)) {
            NumberValue::Integer(integer(digits, 10)?)
        } else {
            NumberValue::Float(float(digits, suffix)?)
        };
        (suffix, value)
    };
    Some(DecodedNumber {
        value,
        suffix: (!suffix.is_empty()).then_some(NameText(suffix)),
    })
}

/// The integer `digits` stand for in `radix`, each `_` skipped; none without a digit, with a digit outside the radix, or past 128 bits.
fn integer(digits: &str, radix: u32) -> Option<IntegerValue> {
    let mut value: u128 = 0;
    let mut counted = false;
    for character in digits.chars().filter(|character| *character != '_') {
        let digit = character.to_digit(radix)?;
        value = value
            .checked_mul(u128::from(radix))?
            .checked_add(u128::from(digit))?;
        counted = true;
    }
    counted.then_some(IntegerValue(value))
}

/// The binary64 value of the decimal `digits`, each `_` skipped: rounded to `f32` first when the suffix is `f32`, to `f64` when it is
/// `f64` or absent; none for another suffix (`f16` and `f128` have no value here) or for digits that are no number of the width.
fn float(digits: &str, suffix: &str) -> Option<FloatBits> {
    let text: Cow<'_, str> = if digits.contains('_') {
        Cow::Owned(digits.replace('_', ""))
    } else {
        Cow::Borrowed(digits)
    };
    if suffix == F32_SUFFIX {
        text.parse::<f32>()
            .ok()
            .map(|value| FloatBits(f64::from(value).to_bits()))
    } else if suffix.is_empty() || suffix == F64_SUFFIX {
        text.parse::<f64>()
            .ok()
            .map(|value| FloatBits(value.to_bits()))
    } else {
        None
    }
}

/// Where the decimal number at the start of `text` ends, and whether a fraction or an exponent made it a float.
fn decimal_end(text: &str) -> (usize, bool) {
    let bytes = text.as_bytes();
    let digits_from = |from: usize| {
        from + bytes
            .get(from..)
            .unwrap_or_default()
            .iter()
            .take_while(|byte| byte.is_ascii_digit() || **byte == b'_')
            .count()
    };
    let mut end = digits_from(0);
    let mut floating = false;
    if bytes.get(end) == Some(&b'.') {
        end = digits_from(end + 1);
        floating = true;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let signed = end + 1 + usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        let after = digits_from(signed);
        if bytes
            .get(signed..after)
            .is_some_and(|exponent| exponent.iter().any(u8::is_ascii_digit))
        {
            end = after;
            floating = true;
        }
    }
    (end, floating)
}
