//! One compact admission operation for diagnostic and observed project phases.
//! Store reads and name/spelling preparation keep their existing eager order;
//! row-cell allocation belongs to the shared executable invocation owner.
use super::types::{FrontierInput, PreparedFrontiers};
use crate::domain::{
    crossing::types::RefusedFrontier,
    frontiers::service::{decode_input, project_input, project_relation},
    operating::errors::OperatingError,
    table_model::types::Frontier,
    table_words::{
        service::{admit_compact, decode_complete},
        types::DecodeMode,
    },
};
use crate::outbound::facts::{
    errors::FactsError,
    interface::{FactsFinish, FactsWorker},
};
use ryo_language::domain::symbols::{service::places_mut, types::Symbols};
use ryo_values::domain::{
    plan_parts::types::{Input, InputNumber},
    stored_facts::types::Bucket,
};

#[derive(Clone, Copy)]
pub(crate) enum PreparationPolicy<'a> {
    Diagnostic,
    Observations { ready: &'a [bool] },
}
#[derive(Debug, thiserror::Error)]
pub(crate) enum PreparationError {
    #[error(transparent)]
    Facts(#[from] FactsError),
    #[error(transparent)]
    Metadata(#[from] OperatingError),
}

pub(crate) fn prepare(
    program: &crate::application::file_dag::program::facade::PreparedProgram,
    rule: crate::domain::coverage::types::RuleIndex,
    first: Bucket,
    facts: &dyn FactsFinish,
    project: &dyn FactsWorker,
    project_symbols: &Symbols,
    symbols: &mut Symbols,
    file_count: usize,
    policy: PreparationPolicy<'_>,
) -> Result<PreparedFrontiers, PreparationError> {
    let mut prepared = PreparedFrontiers {
        rows: Vec::new(),
        refused: Vec::new(),
    };
    let memory =
        tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE);
    for (place, input) in program.frontiers(rule)?.iter().copied().enumerate() {
        let _input = memory.then(|| tracing::trace_span!(target:"ryo::diagnostic::lifecycle_memory", "cross_input_memory", input_index=place, input=?input).entered());
        if let PreparationPolicy::Observations { ready } = policy {
            if !ready.get(place).copied().ok_or(OperatingError::NotRun)? {
                prepared.refused.push(RefusedFrontier {
                    input,
                    error: OperatingError::ObservationIncomplete {
                        reason: "a producer feeding this retained frontier is unavailable".into(),
                    },
                });
                continue;
            }
        }
        if !program
            .metadata()
            .get(rule.0)
            .and_then(|rule| rule.frontier_capture.get(place))
            .copied()
            .ok_or(OperatingError::NotRun)?
        {
            continue;
        }
        if let Some(relation) = project_relation(input) {
            let rows = project.rows(relation)?;
            prepared.rows.push(FrontierInput::Decoded {
                frontier: project_input(rows, input, project_symbols, symbols),
                coverage: crate::application::file_dag::completion::CoverageState::Complete,
            });
            continue;
        }
        // The original fallible store boundary still executes in frontier order,
        // before any query root. This POC changes cell lifetime, not I/O errors.
        let bucket = crate::domain::reading::service::frontier_bucket(first, InputNumber(place));
        let words = facts.kept(bucket)?;
        let coverage = validate_bucket(program, rule, place, first, bucket, file_count, &words)?;
        if memory {
            let bytes = crate::domain::recording::types::retention::value::RetainedBytes::packed(
                &words.words,
            );
            tracing::trace!(target:"ryo::diagnostic::lifecycle_memory", phase="cross_input_packed", owner="packed_input", packed_live_bytes=bytes.packed_live, packed_capacity_bytes=bytes.packed_capacity);
        }
        let mode = match policy {
            PreparationPolicy::Diagnostic => DecodeMode::Permissive,
            PreparationPolicy::Observations { .. } => DecodeMode::Complete,
        };
        if matches!(input, Input::Step(_)) {
            match admit_compact(std::sync::Arc::new(words.words), places_mut(symbols), mode) {
                Ok((table, spelling)) => prepared.rows.push(FrontierInput::Compact {
                    input,
                    table,
                    spelling,
                    coverage,
                }),
                Err(error) => prepared.refused.push(RefusedFrontier {
                    input,
                    error: OperatingError::ObservationIncomplete {
                        reason: error.to_string(),
                    },
                }),
            }
        } else {
            // Non-Step borrowed relation/binding sources keep their existing
            // representation in the bounded POC; no second evaluator exists.
            match mode {
                DecodeMode::Permissive => prepared.rows.push(FrontierInput::Decoded {
                    frontier: Frontier {
                        input,
                        table: decode_input(&words.words, symbols),
                    },
                    coverage,
                }),
                DecodeMode::Complete => {
                    match decode_complete(&words.words.0, places_mut(symbols)) {
                        Ok(table) => prepared.rows.push(FrontierInput::Decoded {
                            frontier: Frontier { input, table },
                            coverage,
                        }),
                        Err(error) => prepared.refused.push(RefusedFrontier {
                            input,
                            error: OperatingError::ObservationIncomplete {
                                reason: error.to_string(),
                            },
                        }),
                    }
                }
            }
        }
    }
    if memory {
        let compact_rows = prepared
            .rows
            .iter()
            .map(|input| match input {
                FrontierInput::Compact { table, .. } => {
                    crate::domain::table_words::service::compact_rows(table)
                }
                FrontierInput::Decoded { .. } => 0,
            })
            .sum::<usize>();
        let spelling_capacity = prepared
            .rows
            .iter()
            .map(|input| match input {
                FrontierInput::Compact { spelling, .. } => {
                    spelling.capacity() * size_of::<ryo_relations::domain::relations::types::Name>()
                }
                FrontierInput::Decoded { .. } => 0,
            })
            .sum::<usize>();
        tracing::trace!(target:"ryo::diagnostic::lifecycle_memory", phase="compact_inputs_prepared", owner="frontier_descriptors", inputs=prepared.rows.len(), compact_rows, spelling_capacity_bytes=spelling_capacity);
    }
    Ok(prepared)
}

/// Check every range and root before a packed source is made visible to the
/// program. The file number comes from the current run's admitted handover;
/// a stored owner token is never trusted or reconstructed.
pub(crate) fn validate_bucket(
    program: &crate::application::file_dag::program::facade::PreparedProgram,
    rule: crate::domain::coverage::types::RuleIndex,
    input: usize,
    first: Bucket,
    bucket: Bucket,
    file_count: usize,
    payload: &crate::outbound::facts::types::KeptBucket,
) -> Result<crate::application::file_dag::completion::CoverageState, OperatingError> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    let refuse = || OperatingError::ObservationIncomplete {
        reason: "frontier source proof does not match the prepared program".into(),
    };
    let expected = program.rule_digest(rule)?;
    if crate::domain::reading::service::frontier_bucket(first, InputNumber(input)) != bucket {
        return Err(refuse());
    }
    let mut at = 0;
    let mut seen_files = std::collections::BTreeSet::new();
    let mut affected = Vec::new();
    for source in &payload.sources {
        if source.start != at
            || source.length == 0
            || usize::try_from(source.file.0).map_err(|_| refuse())? >= file_count
            || !seen_files.insert(source.file.0)
            || source.proof.version != 1
            || source.proof.rule != u32::try_from(rule.0).map_err(|_| refuse())?
            || source.proof.input != u32::try_from(input).map_err(|_| refuse())?
            || source.proof.rule_digest != expected
            || program.frontiers(rule)?.get(input).is_none()
        {
            return Err(refuse());
        }
        at = at.checked_add(source.length).ok_or_else(refuse)?;
        match &source.proof.coverage {
            FrontierCoverage::Complete => {}
            FrontierCoverage::Partial(scopes) => {
                if scopes.is_empty() || !scopes.windows(2).all(|pair| pair[0] < pair[1]) {
                    return Err(refuse());
                }
                affected.extend(scopes.iter().map(|scope| match scope {
                    FrontierAffected::CurrentFile => Some(source.file),
                    FrontierAffected::Repository => None,
                }));
            }
        }
    }
    if at != payload.words.0.0.len() || (file_count > 0 && payload.sources.is_empty()) {
        return Err(refuse());
    }
    affected.sort_unstable();
    affected.dedup();
    Ok(if affected.is_empty() {
        crate::application::file_dag::completion::CoverageState::Complete
    } else {
        crate::application::file_dag::completion::CoverageState::Partial { files: affected }
    })
}
