//! Inbound implementation delegates projection to the pure domain algorithm.
use super::interface::OperatorsUseCase;
use std::sync::Arc;
use crate::outbound::matching::interface::MatchingPort;
use ryo_matching::inbound::use_cases::matching::types::{GroupName, SearchText};
use crate::domain::capture::types::{CaptureSlots, SlotPattern, SlotSubject};
use crate::domain::equality_join::types::{PositionSlots, PositionTarget};
use crate::domain::project::service;
use crate::domain::reduction::types::{Finishing, GroupCounts, HashScratch};
use crate::domain::replace::types::{ReplacedText, ReplacementText};
use crate::domain::select::types::{PredicateOutcome, RowSelection};
use crate::domain::words::types::WordScratch;
use crate::inbound::use_cases::operators::types::OperatorAnswer;
use crate::inbound::use_cases::operators::types::OperatorCall;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::row_texts::types::CellText;

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
                    table,
                    keys,
                    over,
                    order,
                    HashScratch(hashes),
                    hasher,
                    target,
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::HashValue { datum } => Ok(OperatorAnswer::ValueHash(
                crate::domain::reduction::service::hash_of(datum).0,
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
                crate::domain::reduction::service::finish(
                    GroupCounts(counts),
                    hashes,
                    if digest {
                        Finishing::Digested
                    } else {
                        Finishing::Counted
                    },
                    target,
                );
                Ok(OperatorAnswer::Written)
            }

            OperatorCall::Select {
                row,
                kept,
                holds,
                target,
            } => crate::domain::select::service::select(
                row,
                kept,
                if holds {
                    PredicateOutcome::Holds
                } else {
                    PredicateOutcome::Fails
                },
                target,
            )
            .map(|selection| OperatorAnswer::Selected(selection == RowSelection::Kept)),
            OperatorCall::Words { text, word, each } => {
                crate::domain::words::service::split(CellText(text), WordScratch(word), each);
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Capture {
                pattern,
                text,
                slots,
                each,
            } => {
                crate::domain::capture::service::matches(
                    SlotPattern(pattern),
                    SlotSubject(text),
                    SlotSubject(text),
                    CaptureSlots(slots),
                    each,
                );
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Rank { text, layers } => Ok(OperatorAnswer::Rank(
                crate::domain::rank::service::rank(CellText(text), layers),
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
                            .visit_regex_matches(regex, SearchText(text), &mut |found| each(found.0))
                    },
                    CellText(text),
                    ReplacementText(replacement),
                    ReplacedText(target),
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
                crate::domain::aggregate::service::aggregate(
                    row,
                    keys,
                    Count(count),
                    digest.map(Integer),
                    target,
                )?;
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
                let (regex, text, capture) = (input.regex, input.text, input.capture);
                crate::domain::source_matches::service::occurrences(
                    input,
                    driver,
                    target,
                    &|each| match capture {
                        Some(name) => self
                            .matching
                            .visit_regex_captures(
                                regex,
                                SearchText(text),
                                GroupName(name),
                                &mut |found, group| each(found.0, Some(group.0)),
                            )
                            .map_err(|_| PortableOperatorFault::MissingCapture),
                        None => Ok(self.matching.visit_regex_matches(
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
                    crate::domain::set_rows::types::SetInput::Positions {
                        kind,
                        driver,
                    } => crate::domain::set_rows::service::positions(kind, driver)?,
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
                    } => crate::domain::equality_join::service::positions(
                        PositionSlots(left),
                        driver,
                        PositionTarget(target),
                    )?,
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

impl<F: FnMut(&str)> crate::domain::words::types::WordOutput for F {
    fn word(&mut self, text: &str) {
        self(text);
    }
}
impl<F: FnMut(&[std::ops::Range<usize>])> crate::domain::capture::types::CaptureOutput for F {
    fn captures(&mut self, ranges: &[std::ops::Range<usize>]) {
        self(ranges);
    }
}
