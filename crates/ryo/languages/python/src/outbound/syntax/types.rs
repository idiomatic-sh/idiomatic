//! The types of the `syntax` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The bytes of a Python file, borrowed: what the port reads into a table. Python source is UTF-8; bytes that are not are read as
/// text the lexer could not read, at the offsets they have in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceText<'a>(pub &'a [u8]);

/// The Python source form inferred from a path by the Python parse contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PythonSourceKind {
    /// An executable Python module (`.py`).
    Python,
    /// A type stub (`.pyi`).
    Stub,
}
