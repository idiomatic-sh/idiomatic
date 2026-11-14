//! The names of the `decoded` concept: the room one binary64 spelling and one literal's written digits take, and what that
//! spelling is made of.

/// `Display`'s longest finite binary64 spelling, with its `.0` suffix, fits without a heap allocation.
pub const FLOAT_TEXT_BYTES: usize = 350;
/// Room for a numeric literal's significant spelling, its `0x`, `0o` or `0b` prefix with it: binary64's widest whole value has
/// 1024 bits, so no exact literal writes more digits than this.
pub const WRITTEN_DIGITS_BYTES: usize = 1026;
/// The text `numeric Display only writes ASCII`.
pub(crate) const ASCII_ONLY: &str = "numeric Display only writes ASCII";
/// The suffix that keeps a whole-valued binary64 spelling a float.
pub(crate) const FLOAT_SUFFIX: &[u8] = b".0";
/// The most significant digits a decimal literal is printed back to: `Number.prototype.toPrecision`'s bound.
pub(crate) const PRINTED_DIGITS_LIMIT: usize = 100;
/// The bits one hexadecimal digit writes.
pub(crate) const HEX_DIGIT_BITS: u32 = 4;
/// The bits one octal digit writes.
pub(crate) const OCTAL_DIGIT_BITS: u32 = 3;
/// The bits one binary digit writes.
pub(crate) const BINARY_DIGIT_BITS: u32 = 1;
/// The stored fraction bits of a binary64, below its implicit leading one.
pub(crate) const MANTISSA_BITS: u32 = 52;
/// The stored fraction bits of a binary64, as a mask.
pub(crate) const MANTISSA_MASK: u64 = (1 << MANTISSA_BITS) - 1;
/// The biased exponent field of a binary64, once shifted below its fraction.
pub(crate) const EXPONENT_MASK: u64 = 0x7ff;
/// The bias a binary64's exponent field is stored with.
pub(crate) const EXPONENT_BIAS: i64 = 1023;
