//! The types of the `syntax` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The names of the macros whose arguments the port reads again as the expressions they hold: the domain says which, because which
/// macros take expressions is a fact about the language and not about a library, and the edge gives them to the adapter it builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Expanding<'a>(pub &'a [&'a str]);

/// Rust source, as the bytes of a file, borrowed: what the port reads into a table. Bytes that are not text are the adapter's to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceText<'a>(pub &'a [u8]);
