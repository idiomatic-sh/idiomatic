//! Borrowed source matches contracts; no engine authority.
/// The engine lends exactly one checked source interval and its acquired matcher.
pub struct SourceMatchesInput<'a> {
    pub row: &'a [ryo_relations::domain::relations::types::Value],
    pub source: LineBytes<'a>,
    pub base: SourceBase,
    pub text: ryo_values::domain::row_texts::types::CellText<'a>,
    pub line: BaseLine,
    pub regex: &'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
    pub capture: CaptureRequest<'a>,
    pub capture_text: ryo_values::domain::query_operators::types::TextOutput,
    pub capture_line: ryo_values::domain::query_operators::types::LineOutput,
}

/// The line number of the searched text's first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseLine {
    /// Lines are not numbered.
    Unnumbered,
    /// The first byte sits on this line.
    Starts(ryo_relations::domain::relations::types::Integer),
}

/// The named capture group the occurrences are narrowed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureRequest<'a> {
    /// Only the whole occurrence is emitted.
    Whole,
    /// The occurrence's named group is emitted too.
    Group(ryo_values::domain::row_texts::types::CellText<'a>),
}

/// Engine-owned symbol storage and row delivery. No acquisition or publication API is exposed.
pub trait TextRowDriver {
    fn intern(&mut self, text: ryo_values::domain::row_texts::types::CellText<'_>) -> ryo_relations::domain::relations::types::Value;
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), ContractError>;
}

use crate::domain::contract::errors::ContractError;

/// Bytes of the searched text or of the physical source it was cut from.
#[derive(Clone, Copy, Debug)]
pub struct LineBytes<'a>(pub &'a [u8]);

/// Where the searched text starts inside the physical source, in bytes.
#[derive(Clone, Copy, Debug)]
pub struct SourceBase(pub usize);

/// A byte position inside the searched text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LineCursor(pub usize);

/// A byte offset inside the searched text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OccurrenceByte(pub usize);

/// A half-open interval of bytes inside the searched text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OccurrenceSpan {
    /// The first byte of the interval.
    pub start: OccurrenceByte,
    /// The byte after the last of the interval.
    pub end: OccurrenceByte,
}

/// Whether the last byte read was a carriage return that has not yet ended its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CarriageReturn {
    /// A carriage return ends its line unless a line feed follows.
    Pending,
    /// No carriage return awaits a line feed.
    Clear,
}
