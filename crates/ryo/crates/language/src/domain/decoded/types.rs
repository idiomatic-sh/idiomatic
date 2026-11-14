//! The types of the `decoded` concept: a binary64 value a parser decoded, and the room its one spelling is written in.

use super::constants::FLOAT_TEXT_BYTES;

/// The bits of a binary64 float a parser decoded.
///
/// # Examples
/// ```
/// use ryo_language::domain::decoded::types::FloatBits;
/// let one = FloatBits(1.0_f64.to_bits());
/// assert_eq!(f64::from_bits(one.0).to_bits(), 1.0_f64.to_bits());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatBits(pub u64);

/// Bounded stack storage for the spelling of one binary64 float.
///
/// # Examples
/// ```
/// use ryo_language::domain::decoded::constants::FLOAT_TEXT_BYTES;
/// use ryo_language::domain::decoded::types::FloatBuffer;
/// let buffer = FloatBuffer([0; FLOAT_TEXT_BYTES]);
/// assert_eq!(buffer.0.len(), FLOAT_TEXT_BYTES);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FloatBuffer(pub [u8; FLOAT_TEXT_BYTES]);
