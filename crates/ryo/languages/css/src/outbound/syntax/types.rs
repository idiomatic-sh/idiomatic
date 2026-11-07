//! The types of the `syntax` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The bytes of a CSS file, borrowed: what the port reads into a table. The language reads CSS as UTF-8; bytes that are not are
/// part of the file the parser could not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceText<'a>(pub &'a [u8]);
