//! The names of the `model` concept: the prefixes the TOML grammar writes an integer's base with.

/// The prefix of each base an integer may be written in other than ten, with the base (TOML 1.1.0, "Integer": `0x`, `0o`, `0b`).
pub(crate) const RADIX_PREFIXES: [(&str, i64); 3] = [("0x", 16), ("0o", 8), ("0b", 2)];

/// The base of an integer written without a prefix.
pub(crate) const DECIMAL: i64 = 10;
