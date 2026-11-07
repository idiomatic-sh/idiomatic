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
        column = advance_column(column, CandidateText(character.encode_utf8(&mut [0; 4])), tab)?;
        if column.0 > maximum.0 {
            return Ok(Fit::Over);
        }
    }
    Ok(Fit::Within)
}
