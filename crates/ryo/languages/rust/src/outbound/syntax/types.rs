//! The types of the `syntax` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The names of the built-in macros whose arguments are never code (`stringify!`, `concat!`, `env!`, `cfg!`): the port reads the
/// arguments of every other macro call again. The domain says which, because which macros take no code is a fact about the language
/// and not about a library, and the edge gives them to the adapter it builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpaqueMacros<'a>(pub &'a [&'a str]);

/// Rust source, as the bytes of a file, borrowed: what the port reads into a table. Bytes that are not text are the adapter's to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceText<'a>(pub &'a [u8]);

/// One step the parser took, as the table's writer is told of it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Written {
    /// A node begins at the offset.
    Enter(ryo::languages::KindBits, ryo::languages::Offset),
    /// The node last entered ends at the offset.
    Leave(ryo::languages::Offset),
    /// A token spans the offsets and has the text at the place of the tape's texts, if it stands for one.
    Token(ryo::languages::KindBits, ryo::languages::Offset, ryo::languages::Offset, Option<(u32, u32)>),
    /// The node last entered has a property with the text at the place.
    Property(ryo::languages::PropertyId, (u32, u32)),
    /// The parser could not read the span; its message is the text at the place.
    Diagnostic((u32, u32), ryo::languages::Offset, ryo::languages::Offset),
    /// What follows is the expansion of the call at the position.
    Adopt(ryo::languages::Position),
    /// What follows is no longer an expansion.
    Release,
}

/// The parser's steps for one file and the texts they carry.
#[derive(Debug, Default)]
pub(crate) struct Tape {
    /// The steps in order.
    pub(crate) steps: Vec<Written>,
    /// The texts the steps carry, end to end.
    pub(crate) texts: String,
}

