//! The `decoded` service: free functions named by verb that read the text the language's library decoded for an element, with no IO
//! and no ports.

use super::types::{FloatBits, FloatBuffer};
use crate::domain::elements::service::engine;
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::Position;
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
/// Never: the longest finite spelling and its suffix fit [`super::constants::FLOAT_TEXT_BYTES`], and `Display` writes only ASCII.
///
/// # Examples
/// ```
/// use ryo_language::domain::decoded::constants::FLOAT_TEXT_BYTES;
/// use ryo_language::domain::decoded::service::binary64_text;
/// use ryo_language::domain::decoded::types::{FloatBits, FloatBuffer};
/// let mut buffer = FloatBuffer([0; FLOAT_TEXT_BYTES]);
/// assert_eq!(binary64_text(FloatBits(1.0_f64.to_bits()), &mut buffer).0, "1.0");
/// ```
pub fn binary64_text(bits: FloatBits, buffer: &mut FloatBuffer) -> NameText<'_> {
    let _ = (bits, buffer);
    unimplemented!("the move of the spelling lands with the next commit")
}
