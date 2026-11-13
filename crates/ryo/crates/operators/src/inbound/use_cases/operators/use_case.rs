//! Inbound implementation delegates projection to the pure domain algorithm.
use super::interface::OperatorsUseCase;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::project::service;
use crate::domain::source_matches::types::CaptureRequest;
use crate::inbound::use_cases::operators::types::OperatorAnswer;
use crate::inbound::use_cases::operators::types::OperatorCall;
use crate::outbound::matching::interface::MatchingPort;
use ryo_matching::inbound::use_cases::matching::types::{GroupName, SearchText, TextSpan};
use ryo_values::domain::row_texts::types::CellText;
use std::sync::Arc;

/// Capability implementation over the matching port; the edge composes it once.
#[derive(Clone)]
pub struct OperatorsUseCaseImpl {
    /// Finds the occurrences of a prepared regex in a text.
    matching: Arc<dyn MatchingPort>,
}

impl std::fmt::Debug for OperatorsUseCaseImpl {
    /// The use case for a debug print: its port is a trait object with no `Debug`, so it names only itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl OperatorsUseCaseImpl {
    /// A semantic capability composed once at an edge, with the port that finds regex occurrences.
    #[must_use]
    pub fn new(matching: Arc<dyn MatchingPort>) -> Self {
        Self { matching }
    }
}

impl OperatorsUseCase for OperatorsUseCaseImpl {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match call {
            OperatorCall::Evaluate {
                operation,
                input,
                views,
                driver,
                scratch,
            } => {
                crate::domain::evaluation::service::evaluate(
                    operation, input, views, driver, scratch, self,
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::ReduceCount {
                table,
                keys,
                distinct,
                order,
                values,
                target,
            } => {
                crate::domain::reduction::service::count_groups(
                    table, keys, distinct, order, values, target,
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::ReduceDigest {
                table,
                keys,
                over,
                order,
                hashes,
                hasher,
                target,
            } => {
                crate::domain::reduction::service::digest_groups(
                    table, keys, over, order, hashes, hasher, target,
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::HashValue { datum } => Ok(OperatorAnswer::ValueHash(
                crate::domain::reduction::service::hash_of(datum),
            )),
            OperatorCall::CheckAggregation {
                program,
                kinds,
                order,
                target,
            } => {
                crate::domain::reduction::service::check_program(program, kinds, order, target)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::StreamAggregation {
                program,
                row,
                key,
                driver,
            } => {
                crate::domain::reduction::service::push(program, row, key, driver)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::FinishAggregation {
                counts,
                hashes,
                digest,
                target,
            } => {
                crate::domain::reduction::service::finish(counts, hashes, digest, target);
                Ok(OperatorAnswer::Written)
            }

            OperatorCall::Select {
                row,
                kept,
                holds,
                target,
            } => crate::domain::select::service::select(row, kept, holds, target)
                .map(OperatorAnswer::Selected),
            OperatorCall::Words { text, word, each } => {
                crate::domain::words::service::split(text, word, each);
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Capture {
                pattern,
                text,
                slots,
                each,
            } => {
                crate::domain::capture::service::matches(pattern, text, text, slots, each);
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Rank { text, layers } => Ok(OperatorAnswer::Rank(
                crate::domain::rank::service::rank(text, layers),
            )),
            OperatorCall::Replace {
                regex,
                text,
                replacement,
                target,
            } => {
                crate::domain::replace::service::replace_compiled(
                    &|each| {
                        self.matching
                            .visit_regex_matches(regex, SearchText(text.0), &mut |found| {
                                each(found.0)
                            })
                    },
                    text,
                    replacement,
                    target,
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Aggregate {
                row,
                keys,
                count,
                digest,
                target,
            } => {
                crate::domain::aggregate::service::aggregate(row, keys, count, digest, target)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::OrderedMatch {
                output,
                driver,
                target,
            } => {
                crate::domain::ordered_match::service::pairs(output, driver, target)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::SourceMatches {
                input,
                driver,
                target,
            } => {
                let (regex, text, capture) = (input.regex, input.text.0, input.capture);
                crate::domain::source_matches::service::occurrences(
                    input,
                    driver,
                    target,
                    &|each| match capture {
                        CaptureRequest::Group(name) => self
                            .matching
                            .visit_regex_captures(
                                regex,
                                SearchText(text),
                                GroupName(name.0),
                                &mut |found, group| each(found.0, Some(group.0)),
                            )
                            .map_err(|_| PortableOperatorFault::MissingCapture),
                        CaptureRequest::Whole => Ok(self.matching.visit_regex_matches(
                            regex,
                            SearchText(text),
                            &mut |found| each(found.0, None),
                        )),
                    },
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::OrderedExistence {
                partition,
                order,
                extremum,
                driver,
                scratch,
            } => {
                crate::domain::ordered_existence::service::flag(
                    partition, order, extremum, driver, scratch,
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::SetRow { input } => {
                match input {
                    crate::domain::set_rows::types::SetInput::Values {
                        kind,
                        row,
                        left_on,
                        driver,
                        key,
                    } => crate::domain::set_rows::service::values(kind, row, left_on, driver, key)?,
                    crate::domain::set_rows::types::SetInput::Positions { kind, driver } => {
                        crate::domain::set_rows::service::positions(kind, driver)?
                    }
                }
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::EqualityJoin { input } => {
                match input {
                    crate::domain::equality_join::types::JoinInput::Values {
                        left,
                        left_on,
                        output,
                        view,
                        driver,
                        key,
                        target,
                    } => crate::domain::equality_join::service::join_row(
                        left, left_on, output, view, driver, key, target,
                    )?,
                    crate::domain::equality_join::types::JoinInput::Positions {
                        left,
                        driver,
                        target,
                    } => crate::domain::equality_join::service::positions(left, driver, target)?,
                }
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Project {
                row,
                columns,
                target,
            } => {
                service::project(row, columns, target)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Append {
                row,
                appended,
                target,
            } => {
                crate::domain::append::service::append(row, appended, target);
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::FitsRendered {
                rendered,
                prefix,
                maximum,
                tab_width,
            } => crate::domain::fits_rendered::service::fits(rendered, prefix, maximum, tab_width)
                .map(|fit| {
                    OperatorAnswer::Selected(
                        if fit == crate::domain::fits_rendered::types::Fit::Within {
                            crate::domain::select::types::RowSelection::Kept
                        } else {
                            crate::domain::select::types::RowSelection::Dropped
                        },
                    )
                }),
            OperatorCall::Cycles { graph, scratch } => {
                crate::domain::cycles::service::cycles(graph, scratch);
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::TableRows { tables, keys, each } => {
                crate::domain::table_rows::service::rows(tables, keys, each)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::RenderBatch { answers, each } => {
                crate::domain::render_batch::service::deliver(answers, each)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::SpanBounds { row, column } => {
                crate::domain::span_bounds::service::bounds(row, column)
                    .map(OperatorAnswer::SpanBounds)
            }
            OperatorCall::SpanBetween {
                row,
                start,
                end,
                source_len,
            } => crate::domain::span_between::service::between(row, start, end, source_len)
                .map(OperatorAnswer::SpanBetween),
        }
    }
}

impl crate::domain::contract::types::Occurrences for OperatorsUseCaseImpl {
    fn matches(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::types::CompiledRegex,
        text: SearchText<'_>,
        each: &mut dyn FnMut(TextSpan) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        self.matching.visit_regex_matches(regex, text, each)
    }
    fn captures(
        &self,
        regex: &ryo_matching::inbound::use_cases::matching::types::CompiledRegex,
        text: SearchText<'_>,
        group: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, PortableOperatorFault> {
        self.matching
            .visit_regex_captures(regex, text, group, each)
            .map_err(|_| PortableOperatorFault::MissingCapture)
    }
}

impl<F: FnMut(&str)> crate::domain::words::types::WordOutput for F {
    fn word(&mut self, text: CellText<'_>) {
        self(text.0);
    }
}
impl<F: FnMut(&[std::ops::Range<usize>])> crate::domain::capture::types::CaptureOutput for F {
    fn captures(&mut self, ranges: crate::domain::capture::types::CaptureSpans<'_>) {
        self(ranges.0);
    }
}
impl<
    F: FnMut(
        ryo_values::domain::rows::types::RowIndex,
        ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError>,
> crate::domain::render_batch::types::RenderedOutput for F
{
    fn text(
        &mut self,
        position: ryo_values::domain::rows::types::RowIndex,
        text: ryo_values::domain::row_texts::types::CellText<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError> {
        self(position, text)
    }
}
impl<
    F: FnMut(
        crate::domain::table_rows::types::RowTexts<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError>,
> crate::domain::table_rows::types::TableRowOutput for F
{
    fn row(
        &mut self,
        texts: crate::domain::table_rows::types::RowTexts<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError> {
        self(texts)
    }
}
