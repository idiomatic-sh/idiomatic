//! Physical occurrence span, capture and line semantics over one borrowed checked interval.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::{
    BaseLine, BaseLineColumn, CaptureRequest, CarriageReturn, CheckedSource, LineBytes, LineCursor,
    OccurrenceByte, OccurrenceSpan, SearchedGroup, SearchedSpan, SourceBase, SourceMatchesInput,
    SourceMatchesParts, TextRowDriver,
};
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText};
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::row_texts::types::CellText;
use std::ops::ControlFlow;
fn advance_line(
    bytes: LineBytes<'_>,
    source: LineBytes<'_>,
    base: SourceBase,
    cursor: &mut LineCursor,
    to: LineCursor,
    line: &mut Integer,
    pending_cr: &mut CarriageReturn,
) -> Result<(), PortableOperatorFault> {
    for &byte in bytes
        .0
        .get(cursor.0..to.0)
        .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?
    {
        if *pending_cr == CarriageReturn::Pending {
            line.0 = line
                .0
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
            *pending_cr = CarriageReturn::Clear;
            if byte == b'\n' {
                continue;
            }
        }
        if byte == b'\r' {
            *pending_cr = CarriageReturn::Pending;
        } else if byte == b'\n' {
            line.0 = line
                .0
                .checked_add(1)
                .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        }
    }
    *cursor = to;
    if *pending_cr == CarriageReturn::Pending && source.0.get(base.0 + to.0) != Some(&b'\n') {
        line.0 = line
            .0
            .checked_add(1)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        *pending_cr = CarriageReturn::Clear;
    }
    Ok(())
}

/// Visits the occurrences of the input's pattern: each whole span with the span of the captured group, when one was asked for.
pub(crate) type VisitOccurrences<'visit> =
    &'visit dyn Fn(
        &mut dyn FnMut(std::ops::Range<usize>, Option<std::ops::Range<usize>>) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, PortableOperatorFault>;

pub(crate) fn occurrences(
    input: SourceMatchesInput<'_>,
    driver: &mut dyn TextRowDriver,
    joined: &mut Vec<Value>,
    visit: VisitOccurrences<'_>,
) -> Result<(), PortableOperatorFault> {
    let SourceMatchesInput {
        row,
        source,
        base,
        text,
        line,
        regex: _,
        capture,
        capture_text,
        capture_line,
    } = input;
    let (source, text) = (source.0, text.0);
    let capture_text = capture_text == TextOutput::Appended;
    let capture_line = capture_line == LineOutput::Appended;
    let mut line = match line {
        BaseLine::Unnumbered => None,
        BaseLine::Starts(first) => Some(first),
    };
    if (capture_text || capture_line) && capture == CaptureRequest::Whole {
        return Err(PortableOperatorFault::WrongOperation);
    }
    if capture_line && line.is_none() {
        return Err(PortableOperatorFault::WrongOperation);
    }
    let mut cursor = LineCursor(0);
    let mut pending_cr = CarriageReturn::Clear;
    let mut emit_one = |whole: OccurrenceSpan,
                        captured: Option<OccurrenceSpan>|
     -> Result<(), PortableOperatorFault> {
        let (start_at, end_at) = (whole.start.0, whole.end.0);
        let mut whole_line = None;
        if let Some(current) = line.as_mut() {
            advance_line(
                LineBytes(text.as_bytes()),
                LineBytes(source),
                base,
                &mut cursor,
                LineCursor(start_at),
                current,
                &mut pending_cr,
            )?;
            whole_line = Some(*current);
            if capture_line && let Some(group) = &captured {
                advance_line(
                    LineBytes(text.as_bytes()),
                    LineBytes(source),
                    base,
                    &mut cursor,
                    LineCursor(group.start.0),
                    current,
                    &mut pending_cr,
                )?;
            }
        }
        let start = base
            .0
            .checked_add(start_at)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        let end = base
            .0
            .checked_add(end_at)
            .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
        let start =
            u32::try_from(start).map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
        let end = u32::try_from(end).map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
        joined.clear();
        joined.extend_from_slice(row);
        joined.push(Value::Span(Span {
            start: Offset(start),
            end: Offset(end),
        }));
        if let Some(current) = whole_line {
            joined.push(Value::Number(current));
        }
        if let Some(group) = captured {
            let (capture_start, capture_end) = (group.start.0, group.end.0);
            let absolute_start = u32::try_from(
                base.0
                    .checked_add(capture_start)
                    .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
            )
            .map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
            let absolute_end = u32::try_from(
                base.0
                    .checked_add(capture_end)
                    .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
            )
            .map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
            joined.push(Value::Span(Span {
                start: Offset(absolute_start),
                end: Offset(absolute_end),
            }));
            if capture_text {
                let captured_text = text
                    .get(capture_start..capture_end)
                    .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
                // Query execution follows provider fact reading. A fresh source-backed
                // symbol would move `sources` past existing derived provider names
                // (including comments), corrupting their text resolution.
                joined.push(driver.intern(CellText(captured_text)));
            }
            if capture_line {
                joined.push(Value::Number(
                    line.ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?,
                ));
            }
        }
        driver
            .emit(joined)
            .map_err(|_| PortableOperatorFault::Interrupted)?;
        Ok(())
    };
    let mut failure = None;
    let _flow = visit(&mut |whole, group| match emit_one(
        OccurrenceSpan {
            start: OccurrenceByte(whole.start),
            end: OccurrenceByte(whole.end),
        },
        group.map(|group| OccurrenceSpan {
            start: OccurrenceByte(group.start),
            end: OccurrenceByte(group.end),
        }),
    ) {
        Ok(()) => ControlFlow::Continue(()),
        Err(error) => {
            failure = Some(error);
            ControlFlow::Break(())
        }
    })?;
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}

/// Emits the occurrences of the acquired matcher inside the span text the engine validated before it
/// acquired the matcher.
pub(crate) fn evaluate(
    row: &[Value],
    shape: super::types::SearchShape<'_>,
    views: super::types::SearchViews<'_>,
    driver: &mut dyn crate::domain::source_matches::types::TextRowsEvaluation,
    joined: &mut Vec<Value>,
    matching: &dyn crate::domain::contract::types::Occurrences,
) -> Result<(), PortableOperatorFault> {
    let super::types::SearchViews {
        source,
        checked,
        regex,
    } = views;
    let text = checked.text.0;
    let capture = shape.capture;
    occurrences(
        SourceMatchesInput {
            row,
            source,
            base: checked.base,
            text: checked.text,
            line: checked.line,
            regex,
            capture,
            capture_text: shape.capture_text,
            capture_line: shape.capture_line,
        },
        driver.text_rows(),
        joined,
        &|each| match capture {
            CaptureRequest::Group(name) => matching.captures(
                regex,
                ryo_matching::inbound::use_cases::matching::types::SearchText(text),
                ryo_matching::inbound::use_cases::matching::types::GroupName(name.0),
                &mut |found, group| each(found.0, Some(group.0)),
            ),
            CaptureRequest::Whole => Ok(matching.matches(
                regex,
                ryo_matching::inbound::use_cases::matching::types::SearchText(text),
                &mut |found| each(found.0, None),
            )),
        },
    )
}

/// The source gate of a search: the span's text and base line, checked before any pattern is compiled; an absent span gives
/// no occurrence and stops the row's later checks.
///
/// # Errors
///
/// `ShapeMismatch` when the span or base-line cell is of another kind; `InvalidSourceMatchSpan` for a span out of order,
/// outside the source or off a character boundary, or a base line that is not positive; `NoSuchColumn` for a missing column.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::source_matches::service::checked_source;
/// use ryo_operators::domain::source_matches::types::{BaseLineColumn, SearchedSpan};
/// use ryo_relations::domain::relations::types::{Offset, Span, Value};
/// use ryo_values::domain::files::types::SourceView;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// let row = [Value::Span(Span { start: Offset(1), end: Offset(3) })];
/// let checked = checked_source(&row, ColumnIndex(0), BaseLineColumn::Unnumbered, SourceView(b"abcd"));
/// assert!(matches!(checked, Ok(SearchedSpan::Checked(source)) if source.text.0 == "bc"));
/// ```
pub fn checked_source<'a>(
    row: &[Value],
    span: ColumnIndex,
    base_line: BaseLineColumn,
    source: SourceView<'a>,
) -> Result<SearchedSpan<'a>, PortableOperatorFault> {
    let cell = |column: ColumnIndex| {
        ryo_values::domain::rows::service::column_in(row, column)
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
    let Value::Span(span) = cell(span)? else {
        return Err(PortableOperatorFault::ShapeMismatch);
    };
    if span == ryo_relations::domain::relations::constants::NO_SPAN {
        return Ok(SearchedSpan::Absent);
    }
    let start =
        usize::try_from(span.start.0).map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
    let end =
        usize::try_from(span.end.0).map_err(|_| PortableOperatorFault::InvalidSourceMatchSpan)?;
    if start > end {
        return Err(PortableOperatorFault::InvalidSourceMatchSpan);
    }
    let bytes = source
        .0
        .get(start..end)
        .ok_or(PortableOperatorFault::InvalidSourceMatchSpan)?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_error| PortableOperatorFault::InvalidSourceMatchSpan)?;
    let line = match base_line {
        BaseLineColumn::Unnumbered => BaseLine::Unnumbered,
        BaseLineColumn::At(column) => {
            let Value::Number(Integer(number)) = cell(column)? else {
                return Err(PortableOperatorFault::ShapeMismatch);
            };
            if number <= 0 {
                return Err(PortableOperatorFault::InvalidSourceMatchSpan);
            }
            BaseLine::Starts(Integer(number))
        }
    };
    Ok(SearchedSpan::Checked(CheckedSource {
        base: SourceBase(start),
        text: CellText(text),
        line,
    }))
}

/// The one lowering of a source search: its columns, its pattern and capture, and what it appends, for the `text_map`
/// kernel.
pub(crate) fn lower(
    span: ColumnIndex,
    base_line: Option<ColumnIndex>,
    pattern: &LiteralText,
    capture: Option<&LiteralText>,
    capture_text: TextOutput,
    capture_line: LineOutput,
) -> SourceMatchesParts {
    SourceMatchesParts {
        span,
        base_line: base_line.map_or(BaseLineColumn::Unnumbered, BaseLineColumn::At),
        pattern: pattern.clone(),
        capture: capture.map_or(SearchedGroup::Whole, |name| {
            SearchedGroup::Group(name.clone())
        }),
        capture_text,
        capture_line,
    }
}

/// Interns the captured texts a search over `row` kept aside, through `intern`, and writes each into its row's text column,
/// in row order from `first`.
pub(crate) fn interned_captures(
    search: &SourceMatchesParts,
    row: &[Value],
    first: ryo_values::domain::rows::types::RowCount,
    interned: &crate::domain::contract::types::TextScratch,
    pending: &crate::domain::contract::types::RangeScratch,
    output: &mut ryo_values::domain::rows::types::Table,
    intern: &mut dyn FnMut(CellText<'_>) -> Value,
) {
    // The captured text is the column after the row, its span, its line when numbered, and the group's span.
    let width = output.kinds.len();
    let column = row.len() + 2 + usize::from(matches!(search.base_line, BaseLineColumn::At(_)));
    for (place, range) in (first.0..).zip(pending.0.iter()) {
        let value = intern(CellText(interned.0.get(range.clone()).unwrap_or_default()));
        if let Some(target) = output.cells.get_mut(place * width + column) {
            *target = value;
        }
    }
}

/// The row refusal of a search, before its span is read: `WrongOperation` for a text or line output with no capture, or a
/// line output with no base line.
pub(crate) fn refused(search: &SourceMatchesParts) -> Result<(), PortableOperatorFault> {
    let captures = matches!(search.capture, SearchedGroup::Group(_));
    let outputs = search.capture_text == TextOutput::Appended || search.capture_line == LineOutput::Appended;
    if outputs && !captures {
        return Err(PortableOperatorFault::WrongOperation);
    }
    if search.capture_line == LineOutput::Appended && search.base_line == BaseLineColumn::Unnumbered {
        return Err(PortableOperatorFault::WrongOperation);
    }
    Ok(())
}

/// The kinds of the columns a search appends to each row: the occurrence's span, its line when numbered, and the group's
/// span, text and line as asked.
pub(crate) fn appended_kinds(search: &SourceMatchesParts) -> Vec<ryo_relations::domain::relations::types::CellKind> {
    use ryo_relations::domain::relations::types::CellKind;
    let mut kinds = vec![CellKind::Span];
    if matches!(search.base_line, BaseLineColumn::At(_)) {
        kinds.push(CellKind::Number);
    }
    if matches!(search.capture, SearchedGroup::Group(_)) {
        kinds.push(CellKind::Span);
        if search.capture_text == TextOutput::Appended {
            kinds.push(CellKind::Symbol);
        }
        if search.capture_line == LineOutput::Appended {
            kinds.push(CellKind::Number);
        }
    }
    kinds
}
