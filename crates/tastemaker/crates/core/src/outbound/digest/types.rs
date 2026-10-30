//! The types of the `digest` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The byte strings a digest is asked of, in order, each closed by a zero byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DigestParts<'a>(pub &'a [&'a [u8]]);
