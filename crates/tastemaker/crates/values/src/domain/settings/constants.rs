//! The constants of the `settings` concept.

/// How many bytes of the executable are hashed at a time: one buffer is read into and hashed again and again, so a
/// 20 MB `tm` is never held whole, and the chunks fall at the same offsets in every process.
pub const CHUNK: usize = 1 << 20;
