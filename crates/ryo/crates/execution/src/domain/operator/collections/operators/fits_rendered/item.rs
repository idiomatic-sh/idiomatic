//! A pure width calculation over the source prefix and an already rendered candidate.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::{Operation, OperationKind};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::cell;
use unicode_width::UnicodeWidthChar;

fn advance_column(column: usize, text: &str, tab_width: usize) -> Result<usize, OperatingError> {
    let mut column = column;
    for character in text.chars() {
        column = if character == '\t' {
            column
                .checked_div(tab_width)
                .and_then(|stop| stop.checked_add(1))
                .and_then(|stop| stop.checked_mul(tab_width))
                .ok_or(OperatingError::InvalidSpan)?
        } else {
            column
                .checked_add(UnicodeWidthChar::width(character).unwrap_or(0))
                .ok_or(OperatingError::InvalidSpan)?
        };
    }
    Ok(column)
}

fn fits(
    text: &str,
    prefix: &str,
    maximum: usize,
    tab_width: usize,
) -> Result<bool, OperatingError> {
    let mut column = advance_column(0, prefix, tab_width)?;
    if column > maximum {
        return Ok(false);
    }
    let mut previous_cr = false;
    for character in text.chars() {
        if character == '\r' {
            column = 0;
            previous_cr = true;
            continue;
        }
        if character == '\n' {
            if !previous_cr {
                column = 0;
            }
            previous_cr = false;
            continue;
        }
        previous_cr = false;
        column = advance_column(column, character.encode_utf8(&mut [0; 4]), tab_width)?;
        if column > maximum {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) struct FitsRenderedItem;

impl OperatorsItem for FitsRenderedItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::FitsRendered,
            operation,
            inputs,
            bindings,
        )
    }

    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let Operation::FitsRendered {
            texts: columns,
            literals,
            span,
            max_line,
            tab_width,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Value::Span(span) = cell(row, *span)? else {
            return Err(OperatingError::ShapeMismatch);
        };
        let source =
            std::str::from_utf8(lease.views.source.0).map_err(|_| OperatingError::InvalidSpan)?;
        let start = usize::try_from(span.start.0).map_err(|_| OperatingError::InvalidSpan)?;
        let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSpan)?;
        if span == NO_SPAN || start > end || source.get(start..end).is_none() {
            return Err(OperatingError::InvalidSpan);
        }
        let line_start = source[..start].rfind(['\r', '\n']).map_or(0, |at| at + 1);
        let prefix = &source[line_start..start];
        let texts = Texts {
            symbols: lease.derived.as_ref(),
            source: lease.views.source,
            paths: lease.views.paths,
            arenas: lease.views.arenas,
        };
        if literals.len() != columns.len() + 1 {
            return Err(OperatingError::ShapeMismatch);
        }
        let mut rendered = std::mem::take(&mut lease.workspace.text);
        rendered.clear();
        for (literal, column) in literals.iter().zip(columns) {
            rendered.push_str(&literal.0);
            let Datum::Text(value) = datum(cell(row, *column)?, &texts) else {
                lease.workspace.text = rendered;
                return Err(OperatingError::ShapeMismatch);
            };
            rendered.push_str(value.0);
        }
        if let Some(last) = literals.last() {
            rendered.push_str(&last.0);
        }
        let maximum = usize::try_from(*max_line).map_err(|_| OperatingError::WrongBinding)?;
        let tab_width = usize::try_from(*tab_width).map_err(|_| OperatingError::WrongBinding)?;
        if tab_width == 0 {
            return Err(OperatingError::WrongBinding);
        }
        let answer = fits(&rendered, prefix, maximum, tab_width);
        lease.workspace.text = rendered;
        if answer? {
            emit.row(lease, row)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::fits;

    #[test]
    fn physical_width_counts_tabs_unicode_and_all_lines() {
        assert!(fits("x", "\t", 5, 4).unwrap());
        assert!(!fits("xx", "\t", 5, 4).unwrap());
        assert!(fits("界e\u{301}", "", 3, 4).unwrap());
        assert!(!fits("x\n123456", "", 5, 4).unwrap());
        assert!(fits("123456\r\nx", "", 6, 4).unwrap());
    }
}
