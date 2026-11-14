//! The text the `ra_ap_syntax` adapter's code uses.

/// The macro whose arguments are also separated by a semicolon and are not named.
pub(super) const VEC: &str = "vec";

/// The macro whose arguments after the first are a pattern.
pub(super) const MATCHES: &str = "matches";

/// A carriage return and a line feed, which a raw string reads as one line feed.
pub(super) const CRLF: &str = "\r\n";

/// A line feed.
pub(super) const LF: &str = "\n";

/// What a byte that is not part of a character is read as: one byte the lexer reports, so the offsets of the rest do not move.
pub(super) const UNREADABLE: char = '\u{1}';

/// The suffixes that make a numeric literal an integer of a type: the Rust Reference, Integer literals.
pub(super) const INTEGER_SUFFIXES: [&str; 12] = [
    "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "u128", "i128", "usize", "isize",
];

/// The suffix of a 32-bit float, whose value is rounded to `f32` before it is written as binary64.
pub(super) const F32_SUFFIX: &str = "f32";

/// The suffix of a 64-bit float.
pub(super) const F64_SUFFIX: &str = "f64";

/// The prefixes of the integer literals in another base than ten, each with its base.
pub(super) const BASES: [(&str, u32); 3] = [("0x", 16), ("0o", 8), ("0b", 2)];
