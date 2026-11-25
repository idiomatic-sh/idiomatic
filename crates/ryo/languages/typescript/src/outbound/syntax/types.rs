//! The types of the `syntax` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The bytes of a typescript file, borrowed: what the port reads into a table. The language reads typescript as UTF-8; bytes that are not are
/// part of the file the parser could not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceText<'a>(pub &'a [u8]);

/// What a walk of a file leaves besides the numbered table: what the fillers of the language read from the parse.
#[derive(Debug, Default)]
pub(crate) struct Walked {
    /// The control-flow facts, when the walk was asked for them.
    pub(crate) flow: Option<crate::domain::file_view::types::FlowFacts>,
    /// What the library made of each written carrier's pattern.
    pub(crate) patterns: Vec<crate::domain::file_view::types::CarrierFacts>,
    /// The values the parser decoded for tokens.
    pub(crate) decoded: crate::domain::file_view::types::Decoded,
}

/// What a walk keeps for the fillers besides the table.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Wants {
    /// The control-flow facts.
    pub(crate) flow: bool,
    /// The patterns, written under their carriers.
    pub(crate) patterns: bool,
}
