//! The types of the `digest` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The bytes a digest is asked of, borrowed: a file's contents, or a text's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DigestBytes<'a>(pub &'a [u8]);

/// The words a digest is asked of, borrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DigestWords<'a>(pub &'a [u64]);
