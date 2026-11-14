//! Borrowed source matches contracts; no engine authority.
/// The engine lends exactly one checked source interval and its acquired matcher.
pub struct SourceMatchesInput<'a> {
    pub row: &'a [ryo_relations::domain::relations::types::Value],
    pub source: LineBytes<'a>,
    pub base: SourceBase,
    pub text: ryo_values::domain::row_texts::types::CellText<'a>,
    pub line: BaseLine,
    pub regex: &'a ryo_matching::inbound::use_cases::matching::types::CompiledRegex,
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
    fn intern(
        &mut self,
        text: ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> ryo_relations::domain::relations::types::Value;
    /// # Errors
    ///
    /// [`ContractError::Interrupted`] when an engine-retained error interrupts the delivery.
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

/// What an occurrence search reads from its operation: the capture it narrows to and what it emits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SearchShape<'a> {
    /// The named group the occurrences are narrowed to.
    pub capture: CaptureRequest<'a>,
    /// Whether the occurrence's text is emitted.
    pub capture_text: ryo_values::domain::query_operators::types::TextOutput,
    /// Whether the occurrence's line is emitted.
    pub capture_line: ryo_values::domain::query_operators::types::LineOutput,
}

/// What the engine lent an occurrence search: the physical source, the span text it validated before it
/// acquired the matcher, and the matcher.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SearchViews<'a> {
    /// The physical source of the file being read.
    pub source: LineBytes<'a>,
    /// The span text the search was validated against.
    pub checked: CheckedSource<'a>,
    /// The matcher the engine acquired.
    pub regex: &'a ryo_matching::inbound::use_cases::matching::types::CompiledRegex,
}

/// The source text an occurrence search was validated against, before any matcher was acquired.
#[derive(Clone, Copy, Debug)]
pub struct CheckedSource<'a> {
    /// Where the checked text starts inside the physical source.
    pub base: SourceBase,
    /// The checked text.
    pub text: ryo_values::domain::row_texts::types::CellText<'a>,
    /// The line the text starts on.
    pub line: BaseLine,
}

/// An engine driver that lends its symbol and delivery boundary of an occurrence search.
pub trait TextRowsEvaluation: crate::domain::contract::types::RowDriver {
    /// The engine's symbol and delivery boundary of an occurrence search.
    fn text_rows(&mut self) -> &mut dyn TextRowDriver;
}

/// What a source search lowers to: the span and base-line columns, the pattern, the capture and the outputs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceMatchesParts {
    /// The span column searched.
    pub(crate) span: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The line of the span's first byte, when lines are output.
    pub(crate) base_line: BaseLineColumn,
    /// The pattern, compiled once per worker at its first checked row.
    pub(crate) pattern: ryo_values::domain::plan_parts::types::LiteralText,
    /// The named capture, when one is asked for.
    pub(crate) capture: SearchedGroup,
    /// Whether the captured text is appended.
    pub(crate) capture_text: ryo_values::domain::query_operators::types::TextOutput,
    /// Whether the captured line is appended.
    pub(crate) capture_line: ryo_values::domain::query_operators::types::LineOutput,
}

/// Where a source search reads the line its span starts on.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::source_matches::types::BaseLineColumn;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// assert_ne!(BaseLineColumn::At(ColumnIndex(2)), BaseLineColumn::Unnumbered);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseLineColumn {
    /// Lines are not numbered.
    Unnumbered,
    /// The column holding the line.
    At(ryo_values::domain::plan_parts::types::ColumnIndex),
}

/// The named group a source search narrows to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchedGroup {
    /// The whole occurrence only.
    Whole,
    /// The named group.
    Group(ryo_values::domain::plan_parts::types::LiteralText),
}

/// What a search's source gate answered for a row.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::source_matches::service::checked_source;
/// use ryo_operators::domain::source_matches::types::{BaseLineColumn, SearchedSpan};
/// use ryo_relations::domain::relations::types::{Offset, Span, Value};
/// use ryo_values::domain::files::types::SourceView;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// let absent = [Value::Span(Span { start: Offset(u32::MAX), end: Offset(0) })];
/// let searched = checked_source(&absent, ColumnIndex(0), BaseLineColumn::Unnumbered, SourceView(b""));
/// assert!(matches!(searched, Ok(SearchedSpan::Absent)));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum SearchedSpan<'a> {
    /// The span is absent: the row gives no occurrence.
    Absent,
    /// The checked text.
    Checked(CheckedSource<'a>),
}

/// A search's rows written into `output` as they are found, each captured text kept aside in `interned` to be interned once
/// the search is done, so the open file's bytes the search reads stay borrowed until then.
pub(crate) struct DeferredInterning<'a> {
    /// The rows the search writes.
    pub(crate) output: &'a mut ryo_values::domain::rows::types::Table,
    /// The captured texts, one after the other.
    pub(crate) interned: &'a mut crate::domain::contract::types::TextScratch,
    /// The place of each captured text in `interned`, in row order.
    pub(crate) pending: &'a mut crate::domain::contract::types::RangeScratch,
}

impl TextRowDriver for DeferredInterning<'_> {
    fn intern(
        &mut self,
        text: ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> ryo_relations::domain::relations::types::Value {
        let start = self.interned.0.len();
        self.interned.0.push_str(text.0);
        self.pending.0.push(start..self.interned.0.len());
        // Replaced by the interned symbol once the search is done.
        ryo_relations::domain::relations::types::Value::Number(
            ryo_relations::domain::relations::types::Integer(0),
        )
    }

    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), ContractError> {
        self.output.cells.extend_from_slice(row);
        self.output.count = ryo_values::domain::rows::types::RowCount(self.output.count.0 + 1);
        Ok(())
    }
}
