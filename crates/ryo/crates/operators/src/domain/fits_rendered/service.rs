//! Physical line width over a source prefix and an already rendered candidate.
use super::types::{CandidateText, Fit, LineColumn, LinePrefix, LineWidth, TabStop};
use crate::domain::contract::types::PortableOperatorFault;
use unicode_width::UnicodeWidthChar;

fn advance_column(
    column: LineColumn,
    text: CandidateText<'_>,
    tab: TabStop,
) -> Result<LineColumn, PortableOperatorFault> {
    let mut column = column.0;
    for character in text.0.chars() {
        column = if character == '\t' {
            column
                .checked_div(tab.0)
                .and_then(|stop| stop.checked_add(1))
                .and_then(|stop| stop.checked_mul(tab.0))
                .ok_or(PortableOperatorFault::InvalidSpan)?
        } else {
            column
                .checked_add(UnicodeWidthChar::width(character).unwrap_or(0))
                .ok_or(PortableOperatorFault::InvalidSpan)?
        };
    }
    Ok(LineColumn(column))
}

/// Whether every line of `text`, which follows `prefix` on its first line, stays within `maximum` columns.
pub(crate) fn fits(
    text: CandidateText<'_>,
    prefix: LinePrefix<'_>,
    maximum: LineWidth,
    tab: TabStop,
) -> Result<Fit, PortableOperatorFault> {
    if tab.0 == 0 {
        return Err(PortableOperatorFault::WrongOperation);
    }
    let mut column = advance_column(LineColumn(0), CandidateText(prefix.0), tab)?;
    if column.0 > maximum.0 {
        return Ok(Fit::Over);
    }
    let mut previous_cr = false;
    for character in text.0.chars() {
        if character == '\r' {
            column = LineColumn(0);
            previous_cr = true;
            continue;
        }
        if character == '\n' {
            if !previous_cr {
                column = LineColumn(0);
            }
            previous_cr = false;
            continue;
        }
        previous_cr = false;
        column = advance_column(
            column,
            CandidateText(character.encode_utf8(&mut [0; 4])),
            tab,
        )?;
        if column.0 > maximum.0 {
            return Ok(Fit::Over);
        }
    }
    Ok(Fit::Within)
}

/// Keeps a scheduled row whose rendered candidate, after the source text before its span on the same line,
/// stays within the line width.
pub(crate) fn evaluate(
    row: &[ryo_relations::domain::relations::types::Value],
    shape: super::types::RenderedShape<'_>,
    source: crate::domain::source_matches::types::LineBytes<'_>,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    rendered: &mut crate::domain::contract::types::TextScratch,
) -> Result<(), PortableOperatorFault> {
    let span = ryo_values::domain::rows::service::column_in(row, shape.span)
        .copied()
        .ok_or(PortableOperatorFault::NoSuchColumn { index: shape.span })?;
    let LinePrefix(prefix) =
        line_prefix(ryo_values::domain::files::types::SourceView(source.0), span)?;
    if shape.literals.len() != shape.texts.len() + 1 {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    rendered.0.clear();
    for (literal, column) in shape.literals.iter().zip(shape.texts) {
        rendered.0.push_str(&literal.0);
        if driver.text(
            ryo_values::domain::rows::service::column_in(row, *column)
                .copied()
                .ok_or(
                    crate::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                        index: *column,
                    },
                )?,
            rendered,
        ) == crate::domain::contract::types::TextFound::NotText
        {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
    }
    if let Some(last) = shape.literals.last() {
        rendered.0.push_str(&last.0);
    }
    let maximum =
        usize::try_from(shape.max_line.0).map_err(|_| PortableOperatorFault::WrongBinding)?;
    let tab_width =
        usize::try_from(shape.tab_width.0).map_err(|_| PortableOperatorFault::WrongBinding)?;
    if tab_width == 0 {
        return Err(PortableOperatorFault::WrongBinding);
    }
    match fits(
        CandidateText(rendered.0.as_str()),
        LinePrefix(prefix),
        LineWidth(maximum),
        TabStop(tab_width),
    )? {
        Fit::Within => driver.emit(row),
        Fit::Over => Ok(()),
    }
}

/// The one lowering of a fit check: its text columns and literals, its span, its widest line and its tab stop, for the
/// `text_map` kernel.
pub(crate) fn lower(
    texts: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    literals: &[ryo_values::domain::plan_parts::types::LiteralText],
    span: ryo_values::domain::plan_parts::types::ColumnIndex,
    max_line: ryo_values::domain::query_operators::types::MaxLine,
    tab_width: ryo_values::domain::query_operators::types::TabWidth,
) -> super::types::FitsParts {
    super::types::FitsParts {
        texts: texts.to_vec(),
        literals: literals.to_vec(),
        span,
        max_line,
        tab_width,
    }
}

/// The source text on the candidate's line before its span.
///
/// # Errors
///
/// `ShapeMismatch` for a cell that is no span; `InvalidSpan` for an absent span, one out of order or off the source, or a
/// source that is not text.
pub(crate) fn line_prefix<'a>(
    source: ryo_values::domain::files::types::SourceView<'a>,
    span: ryo_relations::domain::relations::types::Value,
) -> Result<LinePrefix<'a>, PortableOperatorFault> {
    let ryo_relations::domain::relations::types::Value::Span(span) = span else {
        return Err(PortableOperatorFault::ShapeMismatch);
    };
    let source = std::str::from_utf8(source.0).map_err(|_| PortableOperatorFault::InvalidSpan)?;
    let start = usize::try_from(span.start.0).map_err(|_| PortableOperatorFault::InvalidSpan)?;
    let end = usize::try_from(span.end.0).map_err(|_| PortableOperatorFault::InvalidSpan)?;
    if span == ryo_relations::domain::relations::constants::NO_SPAN
        || start > end
        || source.get(start..end).is_none()
    {
        return Err(PortableOperatorFault::InvalidSpan);
    }
    let line_start = source
        .get(..start)
        .and_then(|before| before.rfind(['\r', '\n']))
        .map_or(0, |at| at + 1);
    Ok(LinePrefix(
        source.get(line_start..start).unwrap_or_default(),
    ))
}

/// Whether a row's candidate fits: the source line's prefix before its span, then its candidate text, interleaving `parts`'
/// literals with its text columns' texts, then the width; the prefix is kept in `prefix` and the candidate in `candidate`.
///
/// # Errors
///
/// The span's fault; `ShapeMismatch` for literals that do not interleave the texts or a text column with no text;
/// `NoSuchColumn` for a column past the row; `WrongBinding` for a negative width or a zero tab stop.
pub(crate) fn fitted(
    parts: &super::types::FitsParts,
    row: &[ryo_relations::domain::relations::types::Value],
    source: ryo_values::domain::files::types::SourceView<'_>,
    texts: &crate::domain::table_texts::types::Texts<'_>,
    prefix: &mut crate::domain::contract::types::TextScratch,
    candidate: &mut crate::domain::contract::types::TextScratch,
) -> Result<Fit, PortableOperatorFault> {
    use crate::domain::table_texts::types::{Datum, TextView};
    let cell = |column: ryo_values::domain::plan_parts::types::ColumnIndex| {
        ryo_values::domain::rows::service::column_in(row, column)
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
    let LinePrefix(before) = line_prefix(source, cell(parts.span)?)?;
    prefix.0.clear();
    prefix.0.push_str(before);
    if parts.literals.len() != parts.texts.len() + 1 {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    candidate.0.clear();
    for (literal, column) in parts.literals.iter().zip(&parts.texts) {
        candidate.0.push_str(&literal.0);
        match crate::domain::table_texts::service::datum(cell(*column)?, texts) {
            Datum::Text(TextView(found)) => candidate.0.push_str(found),
            Datum::Unlocated
            | Datum::Number(_)
            | Datum::Flag(_)
            | Datum::Numbers(_)
            | Datum::Texts(_) => return Err(PortableOperatorFault::ShapeMismatch),
        }
    }
    if let Some(last) = parts.literals.last() {
        candidate.0.push_str(&last.0);
    }
    let maximum =
        usize::try_from(parts.max_line.0).map_err(|_| PortableOperatorFault::WrongBinding)?;
    let tab_width =
        usize::try_from(parts.tab_width.0).map_err(|_| PortableOperatorFault::WrongBinding)?;
    if tab_width == 0 {
        return Err(PortableOperatorFault::WrongBinding);
    }
    fits(
        CandidateText(&candidate.0),
        LinePrefix(&prefix.0),
        LineWidth(maximum),
        TabStop(tab_width),
    )
}
