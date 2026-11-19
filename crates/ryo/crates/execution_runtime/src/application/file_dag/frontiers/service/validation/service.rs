//! A kept bucket's proof: its provenance checked against the program and the files the rule read, without its words; then,
//! with its words, each frame whole. The file number comes from the run's handover; a stored owner is never trusted.
use crate::application::file_dag::frontiers::constants::PROOF_MISMATCH;
use crate::application::file_dag::frontiers::types::{
    BucketPlace, BucketProvenance, FileAccounting,
};
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::outbound::facts::types::{KeptBucket, KeptWordsLength};
use ryo_execution_domain::domain::completion::service::partial_of_lacks;
use ryo_execution_domain::domain::completion::types::{CoverageState, FileLacks, MissingRows};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::frontier_coverage::service::restored_lacks;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::records::service::is_well_formed;
use ryo_values::domain::records::types::{FrontierCoverage, FrontierShape};
use ryo_values::domain::stored_facts::service::frontier_bucket;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::table_words::service::single_table_frame;
use ryo_values::domain::table_words::types::{Framing, PackedSlice};
use std::collections::BTreeSet;

/// Checks every range and proof of a kept bucket before its rows are read: each frame whole, then its provenance as
/// `validate_sources` checks it. The file number comes from the run's handover; a stored owner is never trusted.
///
/// # Errors
///
/// `PROOF_MISMATCH` when the bucket does not fit the program or the files.
pub(crate) fn validate_bucket(
    program: &PreparedProgram,
    rule: RuleIndex,
    input: InputNumber,
    first: Bucket,
    bucket: Bucket,
    required: &RequiredPartitions,
    payload: &KeptBucket,
) -> Result<CoverageState, OperatingError> {
    let refuse = || OperatingError::ObservationIncomplete {
        reason: PROOF_MISMATCH.into(),
    };
    for source in &payload.sources {
        let end = source
            .start
            .0
            .checked_add(source.length.0)
            .ok_or_else(refuse)?;
        let frame = payload
            .words
            .0
            .0
            .get(source.start.0..end)
            .ok_or_else(refuse)?;
        if single_table_frame(PackedSlice(frame)) != Framing::Whole {
            return Err(refuse());
        }
    }
    validate_sources(
        program,
        BucketPlace {
            rule,
            input,
            first,
            bucket,
        },
        required,
        BucketProvenance {
            sources: &payload.sources,
            length: KeptWordsLength(payload.words.0.0.len()),
        },
    )
}

/// Checks a bucket's provenance without its words: each source the rule's own, one after another from the first word, of a
/// required file and the whole bucket, every required file accounted for. Answers the coverage the sources prove.
///
/// # Errors
///
/// `PROOF_MISMATCH` when the provenance does not fit the program or the files.
pub(crate) fn validate_sources(
    program: &PreparedProgram,
    place: BucketPlace,
    required: &RequiredPartitions,
    kept: BucketProvenance<'_>,
) -> Result<CoverageState, OperatingError> {
    let BucketPlace {
        rule,
        input,
        first,
        bucket,
    } = place;
    let refuse = || OperatingError::ObservationIncomplete {
        reason: PROOF_MISMATCH.into(),
    };
    let expected = program.rule_digest(rule)?;
    let skipped = required.skipped_files(program, rule)?;
    if frontier_bucket(first, input) != bucket || program.frontiers(rule)?.get(input.0).is_none() {
        return Err(refuse());
    }
    // Spike (ryo-fmx3): a required file is looked up in the sorted list by bisection, not by a scan per source.
    let files = required.files();
    let sorted = files.windows(2).all(|pair| pair[0] < pair[1]);
    let is_required = |file: &FileId| {
        if sorted {
            files.binary_search(file).is_ok()
        } else {
            files.contains(file)
        }
    };
    let mut at = 0;
    let mut seen_files = BTreeSet::new();
    let mut affected = Vec::new();
    for source in kept.sources {
        if source.start.0 != at
            || source.length.0 == 0
            || !is_required(&source.file)
            || skipped.contains(&source.file)
            || !seen_files.insert(source.file)
            || source.proof.rule.0 != u32::try_from(rule.0).map_err(|_| refuse())?
            || source.proof.input.0 != u32::try_from(input.0).map_err(|_| refuse())?
            || source.proof.rule_digest != expected
        {
            return Err(refuse());
        }
        at = at.checked_add(source.length.0).ok_or_else(refuse)?;
        match &source.proof.coverage {
            FrontierCoverage::Complete => {}
            FrontierCoverage::Partial(scopes) => {
                if is_well_formed(scopes) == FrontierShape::Malformed {
                    return Err(refuse());
                }
                affected.extend(restored_lacks(source.file, scopes));
            }
        }
    }
    // A file the rule was refused on has no frame: one that carries a frame contradicts the edge.
    let refused = required
        .refused()
        .iter()
        .filter(|file| is_required(file))
        .copied()
        .collect::<BTreeSet<_>>();
    if refused
        .iter()
        .any(|file| seen_files.contains(file) || skipped.contains(file))
    {
        return Err(refuse());
    }
    // Known unavailable files are accounted for without inventing empty frames; they stay attached to the partial answer.
    seen_files.extend(skipped.iter().copied());
    seen_files.extend(refused.iter().copied());
    // A required file with no frame is discharged only when the edge named it inapplicable or refused to this rule.
    seen_files.extend(
        required
            .inapplicable()
            .iter()
            .filter(|file| is_required(file))
            .copied(),
    );
    if at != kept.length.0
        || exact_required_files(required.files(), &seen_files) == FileAccounting::Inexact
    {
        return Err(refuse());
    }
    // A refused file lacks the rule's input, so the answer is partial over it, never complete.
    affected.extend(refused.into_iter().map(|file| FileLacks {
        file: Some(file),
        missing: MissingRows::Any,
    }));
    // A skipped file stays unavailable: it lacks input without making an answer not keyed by it partial.
    Ok(if !affected.is_empty() {
        affected.extend(skipped.iter().copied().map(|file| FileLacks {
            file: Some(file),
            missing: MissingRows::Any,
        }));
        partial_of_lacks(affected)
    } else if skipped.is_empty() {
        CoverageState::Complete
    } else {
        CoverageState::Inapplicable {
            files: skipped.into_iter().collect(),
        }
    })
}

/// Whether `seen` names exactly the `required` files.
fn exact_required_files(required: &[FileId], seen: &BTreeSet<FileId>) -> FileAccounting {
    if seen.len() == required.len() && required.iter().all(|file| seen.contains(file)) {
        FileAccounting::Exact
    } else {
        FileAccounting::Inexact
    }
}
