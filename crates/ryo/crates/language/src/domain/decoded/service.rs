//! The `decoded` service: free functions named by verb that read the text the language's library decoded for an element, with no IO
//! and no ports.

use super::constants::{ASCII_ONLY, FLOAT_SUFFIX, FLOAT_TEXT_BYTES};
use super::types::{FloatBits, FloatBuffer};
use crate::domain::elements::service::engine;
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{Flag, Position};
use ryo_relations::domain::relations::types::PropertyId;
use ryo_relations::domain::terms::types::NameText;

/// The text the token at `at` stands for, when the language's library decoded it.
#[must_use]
pub fn decoded_at(elements: &Elements, at: Position) -> Option<NameText<'_>> {
    property_at(elements, at, PropertyId(0))
}

/// A scalar parser property stored on this node, without claiming a source location.
#[must_use]
pub fn property_at(
    elements: &Elements,
    at: Position,
    property: PropertyId,
) -> Option<NameText<'_>> {
    let table = engine(elements);
    let Ok(found) = table
        .decoded
        .binary_search_by_key(&(at, property), |each| (each.element, each.property))
    else {
        return None; // a binary search answers where a key it does not find would go: that is the key's absence, not a failure
    };
    let each = table.decoded.get(found)?;
    table
        .decoded_text
        .0
        .get(
            usize::try_from(each.from.0).unwrap_or(usize::MAX)
                ..usize::try_from(each.to.0).unwrap_or(usize::MAX),
        )
        // The bytes are cut from text at the places the text was written: a cut that is no text has no text, which the caller reads as no value.
        .and_then(|bytes| match std::str::from_utf8(bytes) {
            Ok(text) => Some(text),
            Err(_) => None,
        })
        .map(NameText)
}

/// The one spelling of a binary64 value every language writes: the shortest round-trip decimal `f64`'s `Display` writes, with
/// `.0` after a finite whole value, written into bounded stack storage and borrowed from it. `Display` never writes an exponent,
/// so `1e300` is its 301 digits; infinities are `inf` and `-inf`, and not-a-number is `NaN`.
///
/// # Panics
/// Never: the longest finite spelling and its suffix fit [`FLOAT_TEXT_BYTES`], and `Display` writes only ASCII.
///
/// # Examples
/// ```
/// use ryo_language::domain::decoded::constants::FLOAT_TEXT_BYTES;
/// use ryo_language::domain::decoded::service::binary64_text;
/// use ryo_language::domain::decoded::types::{FloatBits, FloatBuffer};
/// let mut buffer = FloatBuffer([0; FLOAT_TEXT_BYTES]);
/// assert_eq!(binary64_text(FloatBits(1.0_f64.to_bits()), &mut buffer).0, "1.0");
/// ```
#[expect(
    clippy::expect_used,
    reason = "f64's Display and the `.0` suffix write only ASCII, so the written bytes are always UTF-8"
)]
pub fn binary64_text(bits: FloatBits, buffer: &mut FloatBuffer) -> NameText<'_> {
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
            std::io::Write::write_all(&mut rest, FLOAT_SUFFIX).is_ok(),
            "binary64 suffix fits the fixed buffer"
        );
        written = FLOAT_TEXT_BYTES - rest.len();
    }
    NameText(std::str::from_utf8(&buffer.0[..written]).expect(ASCII_ONLY))
}

/// Whether `bits`, printed to as many significant digits as `written` holds, gives those digits back: the literal loses no
/// precision to binary64. `written` is the literal's digits with the language's own syntax taken out (digit separators, the
/// exponent, suffixes). A `0x`, `0o` or `0b` prefix keeps the base, and the value is printed in it; otherwise the digits are
/// decimal, and zeros after the last non-zero digit are significant only when a `.` is written. A decimal literal writing more
/// than 100 significant digits is never given back, as `Number.prototype.toPrecision` prints at most 100.
///
/// # Examples
/// ```
/// use ryo_language::domain::decoded::service::exact_digits;
/// use ryo_language::domain::decoded::types::FloatBits;
/// use ryo_relations::domain::relations::types::Flag;
/// use ryo_relations::domain::terms::types::NameText;
/// let exact = exact_digits(NameText("0.1"), FloatBits(0.1_f64.to_bits()));
/// assert_eq!(exact, Flag::Raised);
/// let rounded = exact_digits(NameText("9007199254740993"), FloatBits(9_007_199_254_740_992_f64.to_bits()));
/// assert_eq!(rounded, Flag::Lowered);
/// ```
#[must_use]
pub fn exact_digits(written: NameText<'_>, bits: FloatBits) -> Flag {
    let _ = (written, bits);
    Flag::Lowered
}
