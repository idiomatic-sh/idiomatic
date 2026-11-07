//! Borrowed source matches contracts; no engine authority.
/// The engine lends exactly one checked source interval and its acquired matcher.
pub struct SourceMatchesInput<'a> {
    pub row: &'a [ryo_relations::domain::relations::types::Value],
    pub source: &'a [u8],
    pub base: usize,
    pub text: &'a str,
    pub line: Option<i64>,
    pub regex: &'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
    pub capture: Option<&'a str>,
    pub capture_text: bool,
    pub capture_line: bool,
}

/// Engine-owned symbol storage and row delivery. No acquisition or publication API is exposed.
pub trait TextRowDriver {
    fn intern(&mut self, text: &str) -> ryo_relations::domain::relations::types::Value;
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), ContractError>;
}

use crate::domain::contract::errors::ContractError;

/// Bytes of the searched text or of the physical source it was cut from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LineBytes<'a>(pub &'a [u8]);

/// Where the searched text starts inside the physical source, in bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceBase(pub usize);

/// A byte position inside the searched text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LineCursor(pub usize);

/// Whether the last byte read was a carriage return that has not yet ended its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CarriageReturn {
    /// A carriage return ends its line unless a line feed follows.
    Pending,
    /// No carriage return awaits a line feed.
    Clear,
}
