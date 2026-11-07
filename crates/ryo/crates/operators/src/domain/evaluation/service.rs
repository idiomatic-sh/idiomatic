//! The one evaluation of every operation kind: each arm calls the kind's own meaning, once, over the
//! engine's driver port. No kind is evaluated anywhere else.
use super::types::{
    EvaluationInput, EvaluationScratch, EvaluationViews, InvocationDriver, RowRead,
};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::set_rows::types::SetKind;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, InputNumber};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::RowIndex;
use std::ops::ControlFlow;

/// The occurrences of a prepared matcher in a text, found through the matching port the edge composed.
pub(crate) trait Occurrences {
    /// Visits each match of `regex` in `text`, stopping when `each` breaks.
    fn matches(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
        text: &str,
        each: &mut dyn FnMut(std::ops::Range<usize>) -> ControlFlow<()>,
    ) -> ControlFlow<()>;
    /// Visits each match of `regex` in `text` with the span of its named group, stopping when `each` breaks.
    ///
    /// # Errors
    ///
    /// `MissingCapture` when the matcher has no group of that name.
    fn captures(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
        text: &str,
        group: &str,
        each: &mut dyn FnMut(std::ops::Range<usize>, std::ops::Range<usize>) -> ControlFlow<()>,
    ) -> Result<ControlFlow<()>, PortableOperatorFault>;
}

/// One cell of a row, refused as a missing column when the row is narrower.
pub(crate) fn cell(row: &[Value], column: ColumnIndex) -> Result<Value, PortableOperatorFault> {
    usize::try_from(column.0)
        .ok()
        .and_then(|at| row.get(at))
        .copied()
        .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
}

/// The scheduled row of a row-mode evaluation; a population-mode call is not this operation's.
fn row_of(input: EvaluationInput<'_>) -> Result<&[Value], PortableOperatorFault> {
    match input {
        EvaluationInput::Row(row) => Ok(row),
        EvaluationInput::Populations => Err(PortableOperatorFault::WrongOperation),
    }
}

/// A population-mode evaluation; a scheduled row is not this operation's.
fn populations(input: EvaluationInput<'_>) -> Result<(), PortableOperatorFault> {
    match input {
        EvaluationInput::Populations => Ok(()),
        EvaluationInput::Row(_) => Err(PortableOperatorFault::WrongOperation),
    }
}

/// Copies every row of the request's own source, in order: the host materialized them.
fn hosted(
    driver: &mut dyn InvocationDriver,
    row: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let rows = driver.population(InputNumber(0))?;
    for at in 0..rows.0 {
        if driver.population_row(InputNumber(0), RowIndex(at), row) == RowRead::Read {
            driver.emit(row)?;
        }
    }
    Ok(())
}

/// Evaluates one operation over its input. The engine owns acquisition, scheduling and publication; the
/// driver lends its resources and takes every output row.
///
/// # Errors
///
/// The kind's own refusal, or `Interrupted` when the driver retained an engine error.
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive match with one arm for each operation kind, each arm the call of that kind's meaning"
)]
pub(crate) fn evaluate(
    operation: &Operation,
    input: EvaluationInput<'_>,
    views: EvaluationViews<'_>,
    driver: &mut dyn InvocationDriver,
    scratch: &mut EvaluationScratch,
    occurrences: &dyn Occurrences,
) -> Result<(), PortableOperatorFault> {
    match operation {
        Operation::Select { kept, .. } => crate::domain::select::service::evaluate(
            row_of(input)?,
            kept,
            driver,
            &mut scratch.other,
        ),
        Operation::Project {
            columns, distinct, ..
        } => crate::domain::project::service::evaluate(
            row_of(input)?,
            columns,
            *distinct,
            driver,
            &mut scratch.other,
        ),
        Operation::Extend { .. } => crate::domain::append::service::evaluate(
            row_of(input)?,
            views.appended,
            driver,
            &mut scratch.row,
        ),
        Operation::Join {
            left_on,
            right_on,
            output,
            view,
            reach,
            ..
        } => crate::domain::equality_join::service::evaluate(
            row_of(input)?,
            crate::domain::equality_join::types::JoinShape {
                left_on,
                right_on,
                output,
                view,
                reach: reach.as_ref(),
            },
            views.cached_reach,
            driver,
            scratch,
        ),
        Operation::Union { .. } => crate::domain::set_rows::service::evaluate(
            SetKind::Union,
            row_of(input)?,
            &[],
            driver,
            &mut scratch.row,
        ),
        Operation::Intersection { .. } => crate::domain::set_rows::service::evaluate(
            SetKind::Intersection,
            row_of(input)?,
            &[],
            driver,
            &mut scratch.row,
        ),
        Operation::Difference { .. } => crate::domain::set_rows::service::evaluate(
            SetKind::Difference,
            row_of(input)?,
            &[],
            driver,
            &mut scratch.row,
        ),
        Operation::Antijoin { left_on, .. } => crate::domain::set_rows::service::evaluate(
            SetKind::Antijoin,
            row_of(input)?,
            left_on,
            driver,
            &mut scratch.row,
        ),
        Operation::GroupCount { keys, .. } => {
            populations(input)?;
            crate::domain::aggregate::service::evaluate(
                keys,
                crate::domain::aggregate::types::Aggregation::Counted,
                driver,
                scratch,
            )
        }
        Operation::GroupDigest { keys, .. } => {
            populations(input)?;
            crate::domain::aggregate::service::evaluate(
                keys,
                crate::domain::aggregate::types::Aggregation::Digested,
                driver,
                scratch,
            )
        }
        Operation::Capture { path, pattern, .. } => crate::domain::capture::service::evaluate(
            row_of(input)?,
            *path,
            crate::domain::capture::types::SlotPattern(&pattern.0),
            driver,
            scratch,
        ),
        Operation::Rank { column, .. } => crate::domain::rank::service::evaluate(
            row_of(input)?,
            *column,
            views.layers,
            driver,
            scratch,
        ),
        Operation::Replace {
            text, replacement, ..
        } => crate::domain::replace::service::evaluate(
            row_of(input)?,
            *text,
            crate::domain::replace::types::ReplacementText(&replacement.0),
            views.regex,
            driver,
            scratch,
            occurrences,
        ),
        Operation::OrderedMatch { output, .. } => {
            populations(input)?;
            crate::domain::ordered_match::service::evaluate(output, driver, &mut scratch.captured)
        }
        Operation::Words { text, .. } => {
            crate::domain::words::service::evaluate(row_of(input)?, *text, driver, scratch)
        }
        Operation::Axis {
            id,
            direction,
            output,
            ..
        } => crate::domain::axis::service::evaluate(
            row_of(input)?,
            *id,
            *direction,
            output,
            views.cached_axis,
            driver,
            &mut scratch.row,
        ),
        Operation::Closure { .. } => {
            populations(input)?;
            crate::domain::closure::service::evaluate(driver, scratch)
        }
        Operation::OrderedDfs {
            root,
            source,
            destination,
            edge,
            root_keys,
            ..
        } => {
            populations(input)?;
            crate::domain::ordered_dfs::service::evaluate(
                crate::domain::ordered_dfs::types::DfsShape {
                    root: *root,
                    edge_columns: [*source, *destination, *edge],
                    root_keys,
                },
                driver,
                scratch,
            )
        }
        Operation::Cycles { .. } => {
            populations(input)?;
            crate::domain::cycles::service::evaluate(driver, &mut scratch.cycles)
        }
        Operation::Rows { parameter, keys } => {
            populations(input)?;
            crate::domain::table_rows::service::evaluate(
                *parameter,
                keys,
                views.bindings,
                driver,
                &mut scratch.row,
            )
        }
        Operation::Hosted { .. } => {
            populations(input)?;
            hosted(driver, &mut scratch.row)
        }
        Operation::SpanBetween { start, end, .. } => {
            let row = row_of(input)?;
            let span = crate::domain::span_between::service::between(
                row,
                *start,
                *end,
                crate::domain::contract::types::SourceByteLength(views.source.0.len()),
            )?;
            crate::domain::append::service::append(
                row,
                std::slice::from_ref(&span),
                &mut scratch.row,
            );
            driver.emit(&scratch.row)
        }
        Operation::SpanBounds { span, .. } => {
            let row = row_of(input)?;
            let bounds = crate::domain::span_bounds::service::bounds(row, *span)?;
            crate::domain::append::service::append(row, &bounds, &mut scratch.row);
            driver.emit(&scratch.row)
        }
        Operation::SourceMatches {
            capture,
            capture_text,
            capture_line,
            ..
        } => crate::domain::source_matches::service::evaluate(
            row_of(input)?,
            crate::domain::source_matches::types::SearchShape {
                capture: capture.as_ref().map_or(
                    crate::domain::source_matches::types::CaptureRequest::Whole,
                    |name| {
                        crate::domain::source_matches::types::CaptureRequest::Group(
                            ryo_values::domain::row_texts::types::CellText(name.0.as_ref()),
                        )
                    },
                ),
                capture_text: *capture_text,
                capture_line: *capture_line,
            },
            views,
            driver,
            &mut scratch.row,
            occurrences,
        ),
        Operation::RenderSyntax { node, .. } => {
            populations(input)?;
            crate::domain::render_batch::service::evaluate(*node, driver, scratch)
        }
        Operation::FitsRendered {
            texts,
            literals,
            span,
            max_line,
            tab_width,
            ..
        } => crate::domain::fits_rendered::service::evaluate(
            row_of(input)?,
            crate::domain::fits_rendered::types::RenderedShape {
                texts,
                literals,
                span: *span,
                max_line: *max_line,
                tab_width: *tab_width,
            },
            views.source,
            driver,
            &mut scratch.text,
        ),
    }
}
