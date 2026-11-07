//! The `lint` service: local free-function helpers, private to this use case.

use super::constants::TASK_ANSWERED;
use super::errors::LintError;
use super::types::MemoryDiagnostics;
use crate::domain::project::types::Projected;
use crate::domain::reporting::service::report_refused;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::types::Unreadable;
use crate::domain::reading::service::buckets_of;
use crate::inbound::use_cases::lint::constants::{
    CROSS_RULE_MEMORY, CROSS_SYMBOLS, FRESH_RECORDS, LIFECYCLE_MEMORY_DIAGNOSTIC, STORE_HANDED,
    STORE_STAGED_DESCRIPTORS, WORKER_ARENAS, WORKER_NOTES,
};
use crate::outbound::lint_store::interface::ReadStore;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, StoredHanded};
use std::sync::Arc;
use {
    crate::domain::storing::types::HandedAgain, ryo_values::domain::stores::types::MadeAgain,
    ryo_values::domain::stores::types::Reuse,
};

/// What a task's slot holds once the workers are done: its answer.
///
/// # Panics
///
/// When no worker answered the task: the workers take tasks until none is left, so a slot left empty is a bug that must crash,
/// never a task quietly dropped from the answer.
#[expect(
    clippy::expect_used,
    reason = "the workers take tasks until none is left; a slot left empty is a bug that must crash, never a task dropped from the answer"
)]
pub(super) fn answered<T>(slot: Option<T>) -> T {
    slot.expect(TASK_ANSWERED)
}

/// The record kept of each of `files`, in their order; none for a file with no record.
pub(super) fn records_of<'a>(files: &[CorpusInput], kept: &'a [Record]) -> Vec<Option<&'a Record>> {
    files
        .iter()
        .map(|file| {
            let CorpusInput::Available(entry) = file else {
                return None;
            };
            match kept.binary_search_by(|record| record.path.cmp(&entry.path)) {
                Ok(at) => kept.get(at),
                // The search answers where the path would go: the file has no record.
                Err(_) => None,
            }
        })
        .collect()
}

/// The buckets a run that makes cross-file steps again reads of what the files handed them, those of the rules made again
/// alone, and what the store kept of them.
pub(super) fn handed_of(
    read: Option<&Arc<dyn ReadStore>>,
    metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
    first_bucket: &[Bucket],
    again: &Reuse,
    producer_buckets: &[Bucket],
) -> Result<HandedAgain, LintError> {
    let mut wanted = match again {
        Reuse::Cross(MadeAgain::These(made)) => {
            BucketsWanted::These(buckets_of(metadata, first_bucket, Some(made)))
        }
        Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) | Reuse::Whole => BucketsWanted::All,
        Reuse::Again => BucketsWanted::These(Vec::new()),
    };
    if let BucketsWanted::These(buckets) = &mut wanted {
        buckets.extend_from_slice(producer_buckets);
        buckets.sort_unstable();
        buckets.dedup();
    }
    let handed = match again {
        Reuse::Cross(_) => read
            .map(|read| read.handed(&wanted))
            .transpose()?
            .unwrap_or_default(),
        Reuse::Whole | Reuse::Again => ReadHanded::default(),
    };
    Ok(HandedAgain { wanted, handed })
}

/// One explicit opt-in gate; no capacity traversal or diagnostic span construction when disabled.
pub(super) fn memory_diagnostics() -> MemoryDiagnostics {
    if tracing::enabled!(target: LIFECYCLE_MEMORY_DIAGNOSTIC, tracing::Level::TRACE) {
        MemoryDiagnostics::Enabled
    } else {
        MemoryDiagnostics::Disabled
    }
}

/// Component capacities are distinct from RSS, allocator retention and unmeasured provider storage.
pub(super) fn observed_memory(
    phase: &'static str,
    owner: &'static str,
    bytes: ryo_values::domain::stores::types::retention::value::RetainedBytes,
) {
    tracing::trace!(target: LIFECYCLE_MEMORY_DIAGNOSTIC, phase, owner,
        objects = bytes.objects.0, metadata_capacity_bytes = bytes.metadata.0,
        packed_live_bytes = bytes.packed_live.0, packed_capacity_bytes = bytes.packed_capacity.0,
        staged_objects = bytes.staged_objects.0, staged_payload_bytes = bytes.staged_payload_bytes.0,
        text_live_bytes = bytes.text_live.0, text_capacity_bytes = bytes.text_capacity.0,
        notes_live_bytes = bytes.notes_live.0, notes_capacity_bytes = bytes.notes_capacity.0,
        findings_capacity_bytes = bytes.findings_capacity.0,
        decoded_live_bytes = bytes.cells_live.0, decoded_capacity_bytes = bytes.cells_capacity.0, decoded_rows = bytes.rows.0,
        places_live_bytes = bytes.places_live.0, places_capacity_bytes = bytes.places_capacity.0);
}

/// Record capacity traversal is disabled on the normal lint path.
pub(super) fn observed_records(phase: &'static str, records: &Vec<Record>) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            FRESH_RECORDS,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::records(records),
        );
    }
}

/// The arena owner survives independently of the normalized text in fresh records.
pub(super) fn observed_arenas(
    phase: &'static str,
    arenas: &Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            WORKER_ARENAS,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::arenas(arenas),
        );
    }
}

/// Observe the cross-file symbols owned here; execution owns input residency.
pub(super) fn observed_cross_symbols(
    phase: &'static str,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        let mut bytes =
            ryo_values::domain::stores::types::retention::value::RetainedBytes::default();
        let places = ryo_language::domain::symbols::service::places_mut(symbols);
        bytes.places_live.0 =
            places.len() * size_of::<ryo_relations::domain::relations::types::NamePlace>();
        bytes.places_capacity.0 =
            places.capacity() * size_of::<ryo_relations::domain::relations::types::NamePlace>();
        let text = ryo_language::domain::names::service::derived_of(symbols);
        bytes.text_live.0 = text.0.len();
        bytes.text_capacity.0 = text.0.capacity();
        observed_memory(phase, CROSS_SYMBOLS, bytes);
    }
}

/// Worker notes are a separate owner from their normalized record copy.
pub(super) fn observed_notes(
    phase: &'static str,
    notes: &crate::domain::workers::types::FreshNotes,
) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            WORKER_NOTES,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::worker_notes(
                &notes.0,
                |entry| &entry.notes,
            ),
        );
    }
}

/// Keep rule identity on decoder events without changing its domain API.
pub(super) fn memory_rule(
    id: &ryo_values::domain::runs::types::RuleId,
    index: usize,
) -> Option<tracing::span::EnteredSpan> {
    (memory_diagnostics() == MemoryDiagnostics::Enabled).then(|| tracing::trace_span!(target: LIFECYCLE_MEMORY_DIAGNOSTIC, CROSS_RULE_MEMORY, rule = %id.0, rule_index = index).entered())
}

/// Do not walk stored payload metadata on ordinary lints.
pub(super) fn observed_stored(phase: &'static str, handed: &StoredHanded) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            STORE_HANDED,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::stored(handed),
        );
    }
}

/// Publication observes staged descriptor capacity without reading spool contents.
pub(super) fn observed_staged(
    phase: &'static str,
    kept: &ryo_values::domain::stores::types::KeptRows,
) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            STORE_STAGED_DESCRIPTORS,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::staged(kept),
        );
    }
}

/// Preserve present and absent project dependencies in the completed report.
pub(super) fn project_dependencies(now: &Projected) -> Vec<super::types::ProjectDependency> {
    now.consulted
        .iter()
        .map(|dependency| super::types::ProjectDependency {
            path: dependency.path.clone(),
            state: super::types::ProjectDependencyState::Present(dependency.digest),
        })
        .chain(
            now.consulted_absent
                .iter()
                .map(|path| super::types::ProjectDependency {
                    path: path.clone(),
                    state: super::types::ProjectDependencyState::Missing,
                }),
        )
        .collect()
}

/// Collect retry paths before adding refused manifests to the final unread report.
pub(super) fn read_outcomes(
    unread: &mut Vec<ryo_values::domain::records::types::Unread>,
    unrecorded: Vec<FilePath>,
    refused: &[ryo_values::domain::runs::types::ManifestRefused],
) -> Vec<FilePath> {
    let mut failed: Vec<FilePath> = unread
        .iter()
        .filter(|each| matches!(each.reason, Unreadable::Failed(_)))
        .map(|each| each.path.clone())
        .collect();
    failed.extend(unrecorded);
    failed.sort_unstable();
    failed.dedup();
    report_refused(unread, refused);
    failed
}

/// Preserve one ordered occurrence of each completed coverage record.
pub(super) fn sort_unique<T>(records: &mut Vec<T>)
where
    T: Ord,
{
    records.sort();
    records.dedup();
}
