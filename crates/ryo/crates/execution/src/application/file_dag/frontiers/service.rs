//! One compact admission operation for diagnostic and observed project phases.
//! Store reads and name/spelling preparation keep their existing eager order;
//! row-cell allocation belongs to the shared executable invocation owner.
use super::types::{FrontierInput, FrontierSource, PackedFrontier, PreparedFrontiers};
use crate::outbound::facts::{
    errors::FactsError,
    interface::{FactsFinish, FactsWorker},
};
use ryo_language::domain::symbols::{service::places_mut, types::Symbols};
use ryo_values::domain::{
    plan_parts::types::{Input, InputNumber},
    stored_facts::types::Bucket,
};
use {
    crate::domain::crossing::types::RefusedFrontier,
    crate::domain::frontiers::service::decode_input,
    crate::domain::frontiers::service::project_input,
    crate::domain::frontiers::service::project_relation,
    crate::domain::operating::errors::OperatingError, crate::domain::table_model::types::Frontier,
    ryo_values::domain::table_words::service::decode_complete,
    ryo_values::domain::table_words::types::DecodeMode,
};

#[derive(Clone, Copy)]
pub(crate) enum PreparationPolicy<'a> {
    Diagnostic,
    Observations { ready: &'a [bool] },
}
#[derive(Debug, thiserror::Error)]
pub enum PreparationError {
    #[error(transparent)]
    Facts(#[from] FactsError),
    #[error(transparent)]
    Metadata(#[from] OperatingError),
}

pub(super) fn prepare(
    program: &crate::application::file_dag::program::preparation::types::PreparedProgram,
    rule: crate::domain::coverage::types::RuleIndex,
    first: Bucket,
    facts: &dyn FactsFinish,
    project: &dyn FactsWorker,
    project_symbols: &Symbols,
    symbols: &mut Symbols,
    required: &crate::application::file_dag::program::admission::RequiredPartitions,
    policy: PreparationPolicy<'_>,
) -> Result<PreparedFrontiers, PreparationError> {
    let roots = &program
        .program()
        .rules()
        .get(rule.0)
        .ok_or(OperatingError::NotRead)?
        .frontier;
    let sources = program
        .frontiers(rule)?
        .iter()
        .copied()
        .enumerate()
        .map(|(at, input)| {
            Ok(FrontierSource {
                input,
                root: roots.get(at).ok_or(OperatingError::NotRead)?.clone(),
            })
        })
        .collect::<Result<Vec<_>, OperatingError>>()?
        .into();
    let mut admitted = Vec::new();
    let mut refused = Vec::new();
    let memory =
        tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE);
    for (place, input) in program.frontiers(rule)?.iter().copied().enumerate() {
        let _input = memory.then(|| tracing::trace_span!(target:"ryo::diagnostic::lifecycle_memory", "cross_input_memory", input_index=place, input=?input).entered());
        if let PreparationPolicy::Observations { ready } = policy {
            if !ready.get(place).copied().ok_or(OperatingError::NotRun)? {
                refused.push(RefusedFrontier {
                    input,
                    error: OperatingError::ObservationIncomplete {
                        reason: "a producer feeding this retained frontier is unavailable".into(),
                    },
                });
                continue;
            }
        }
        if program
            .metadata()
            .get(rule.0)
            .and_then(|rule| rule.frontier_capture.get(place))
            .copied()
            .ok_or(OperatingError::NotRun)?
            == ryo_values::domain::run_metadata::types::FrontierFeed::CrossFile
        {
            continue;
        }
        if let Some(relation) = project_relation(input) {
            let rows = project.rows(relation)?;
            admitted.push(FrontierInput::Decoded {
                frontier: project_input(rows, input, project_symbols, symbols),
                coverage: crate::domain::completion::types::CoverageState::Complete,
            });
            continue;
        }
        // The original fallible store boundary still executes in frontier order,
        // before any query root. This POC changes cell lifetime, not I/O errors.
        let bucket =
            ryo_values::domain::stored_facts::service::frontier_bucket(first, InputNumber(place));
        let words = facts.kept(bucket)?;
        let coverage = validate_bucket(program, rule, place, first, bucket, required, &words)?;
        if words.sources.is_empty() && !required.files().is_empty() {
            refused.push(RefusedFrontier {
                input,
                error: OperatingError::ObservationIncomplete {
                    reason: "every source partition was explicitly skipped".into(),
                },
            });
            continue;
        }
        if memory {
            let bytes = ryo_values::domain::stores::types::retention::value::RetainedBytes::packed(
                &words.words,
            );
            tracing::trace!(target:"ryo::diagnostic::lifecycle_memory", phase="cross_input_packed", owner="packed_input", packed_live_bytes=bytes.packed_live.0, packed_capacity_bytes=bytes.packed_capacity.0);
        }
        let mode = match policy {
            PreparationPolicy::Diagnostic => DecodeMode::Permissive,
            PreparationPolicy::Observations { .. } => DecodeMode::Complete,
        };
        // Decoding waits for the first demand (`admit_all`): the store read, the
        // proof and the refusals above keep their order; only the decode is deferred.
        admitted.push(FrontierInput::Packed(PackedFrontier {
            input,
            words: std::sync::Arc::new(words.words),
            mode,
            coverage,
        }));
    }
    if memory {
        let compact_rows = admitted
            .iter()
            .map(|input| match input {
                FrontierInput::Compact { table, .. } => {
                    ryo_values::domain::compact_table::service::compact_rows(table).0
                }
                FrontierInput::Decoded { .. } | FrontierInput::Packed(_) => 0,
            })
            .sum::<usize>();
        let spelling_capacity = admitted
            .iter()
            .map(|input| match input {
                FrontierInput::Compact { spelling, .. } => {
                    spelling.capacity() * size_of::<ryo_relations::domain::relations::types::Name>()
                }
                FrontierInput::Decoded { .. } | FrontierInput::Packed(_) => 0,
            })
            .sum::<usize>();
        tracing::trace!(target:"ryo::diagnostic::lifecycle_memory", phase="compact_inputs_prepared", owner="frontier_descriptors", inputs=admitted.len(), compact_rows, spelling_capacity_bytes=spelling_capacity);
    }
    Ok(PreparedFrontiers {
        producer: rule,
        sources,
        rows: admitted.into(),
        program: std::sync::Arc::clone(program.program()),
        required: required.clone(),
        refused: refused.into(),
    })
}

/// The first demand of the frontiers: decode what preparation left packed, in place. A failed
/// decode becomes the typed refusal the first demand of that source returns.
pub(in crate::application::file_dag) fn admit_all(
    frontiers: &mut PreparedFrontiers,
    symbols: &mut Symbols,
) {
    let mut rows = Vec::with_capacity(frontiers.rows.len());
    let mut refused = frontiers.refused.to_vec();
    for row in frontiers.rows.iter().cloned() {
        match row {
            FrontierInput::Packed(packed) => match admit(packed, symbols) {
                Ok(admitted) => rows.push(admitted),
                Err(error) => refused.push(error),
            },
            admitted => rows.push(admitted),
        }
    }
    frontiers.rows = rows.into();
    frontiers.refused = refused.into();
}

/// Decode one packed frontier on its first demand. A decode failure is the same typed
/// refusal preparation used to record, now recorded where the frontier is first admitted.
pub(crate) fn admit(
    packed: PackedFrontier,
    symbols: &mut Symbols,
) -> Result<FrontierInput, RefusedFrontier> {
    let PackedFrontier {
        input,
        words,
        mode,
        coverage,
    } = packed;
    let refuse =
        |error: ryo_values::domain::table_words::errors::TableWordsError| RefusedFrontier {
            input,
            error: OperatingError::ObservationIncomplete {
                reason: error.to_string(),
            },
        };
    if matches!(input, Input::Step(_)) {
        return ryo_values::domain::compact_table::service::admit_compact(
            ryo_values::domain::table_words::types::CompactBacking(words),
            places_mut(symbols),
            mode,
        )
        .map(
            |ryo_values::domain::table_words::types::Admitted {
                 table,
                 names: spelling,
             }| FrontierInput::Compact {
                input,
                table: std::sync::Arc::new(table),
                spelling,
                coverage,
            },
        )
        .map_err(refuse);
    }
    // Non-Step borrowed relation/binding sources keep their existing representation.
    match mode {
        DecodeMode::Permissive => Ok(FrontierInput::Decoded {
            frontier: Frontier {
                input,
                table: std::sync::Arc::new(decode_input(&words, symbols)),
            },
            coverage,
        }),
        DecodeMode::Complete => decode_complete(&words.0, places_mut(symbols))
            .map(|table| FrontierInput::Decoded {
                frontier: Frontier {
                    input,
                    table: std::sync::Arc::new(table),
                },
                coverage,
            })
            .map_err(refuse),
    }
}

/// Check every range and root before a packed source is made visible to the
/// program. The file number comes from the current run's admitted handover;
/// a stored owner token is never trusted or reconstructed.
pub(crate) fn validate_bucket(
    program: &crate::application::file_dag::program::preparation::types::PreparedProgram,
    rule: crate::domain::coverage::types::RuleIndex,
    input: usize,
    first: Bucket,
    bucket: Bucket,
    required: &crate::application::file_dag::program::admission::RequiredPartitions,
    payload: &crate::outbound::facts::types::KeptBucket,
) -> Result<crate::domain::completion::types::CoverageState, OperatingError> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    let refuse = || OperatingError::ObservationIncomplete {
        reason: "frontier source proof does not match the prepared program".into(),
    };
    let expected = program.rule_digest(rule)?;
    let skipped = required.skipped_files(program, rule)?;
    if ryo_values::domain::stored_facts::service::frontier_bucket(first, InputNumber(input))
        != bucket
        || program.frontiers(rule)?.get(input).is_none()
    {
        return Err(refuse());
    }
    let mut at = 0;
    let mut seen_files = std::collections::BTreeSet::new();
    let mut affected = Vec::new();
    for source in &payload.sources {
        if source.start != at
            || source.length == 0
            || !required.files().contains(&source.file)
            || skipped.contains(&source.file)
            || !seen_files.insert(source.file.0)
            || source.proof.rule.0 != u32::try_from(rule.0).map_err(|_| refuse())?
            || source.proof.input.0 != u32::try_from(input).map_err(|_| refuse())?
            || source.proof.rule_digest != expected
        {
            return Err(refuse());
        }
        let end = at.checked_add(source.length).ok_or_else(refuse)?;
        let frame = payload.words.0.0.get(at..end).ok_or_else(refuse)?;
        if ryo_values::domain::table_words::service::single_table_frame(
            ryo_values::domain::table_words::types::PackedSlice(frame),
        ) != ryo_values::domain::table_words::types::Framing::Whole
        {
            return Err(refuse());
        }
        at = end;
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
    // Account for known unavailable partitions without inventing encoded empty
    // frames. Their identities remain attached to the admitted partial input.
    seen_files.extend(skipped.iter().map(|file| file.0));
    // A required source with no frame is discharged only when the edge named the file inapplicable to this rule.
    seen_files.extend(
        required
            .inapplicable()
            .iter()
            .filter(|file| required.files().contains(file))
            .map(|file| file.0),
    );
    if at != payload.words.0.0.len() || !exact_required_files(required.files(), &seen_files) {
        return Err(refuse());
    }
    affected.sort_unstable();
    affected.dedup();
    // A skipped file stays unavailable: it lacks input without making an answer not keyed by it partial.
    Ok(if !affected.is_empty() {
        affected.extend(skipped.iter().copied().map(Some));
        affected.sort_unstable();
        affected.dedup();
        crate::domain::completion::types::CoverageState::Partial { files: affected }
    } else if skipped.is_empty() {
        crate::domain::completion::types::CoverageState::Complete
    } else {
        crate::domain::completion::types::CoverageState::Inapplicable {
            files: skipped.into_iter().collect(),
        }
    })
}

fn exact_required_files(
    required: &[ryo_relations::domain::relations::types::FileId],
    seen: &std::collections::BTreeSet<u32>,
) -> bool {
    seen.len() == required.len() && required.iter().all(|file| seen.contains(&file.0))
}

#[cfg(test)]
mod source_proof_test {
    use super::exact_required_files;
    use ryo_relations::domain::relations::types::FileId;

    #[test]
    fn a_complete_bucket_requires_every_declared_project_file() {
        let required = [FileId(0), FileId(1)];
        assert!(!exact_required_files(&required, &[0].into()));
        assert!(exact_required_files(&required, &[0, 1].into()));
        assert!(!exact_required_files(&required, &[0, 1, 2].into()));
    }
}
