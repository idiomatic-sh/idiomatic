//! The `lint` service: local free-function helpers, private to this use case.

use super::constants::TASK_ANSWERED;
use super::errors::LintError;
use crate::domain::reading::service::buckets_of;
use crate::domain::storing::types::{HandedAgain, MadeAgain, Reuse};
use crate::outbound::lint_store::interface::ReadStore;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, StoredHanded};
use std::sync::Arc;

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
    plans: &[Plan],
    first_bucket: &[Bucket],
    again: &Reuse,
) -> Result<HandedAgain, LintError> {
    let wanted = match again {
        Reuse::Cross(MadeAgain::These(made)) => {
            BucketsWanted::These(buckets_of(plans, first_bucket, Some(made)))
        }
        Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) | Reuse::Whole => BucketsWanted::All,
        Reuse::Again => BucketsWanted::These(Vec::new()),
    };
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
pub(super) fn memory_enabled() -> bool {
    tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE)
}

/// Component capacities are distinct from RSS, allocator retention and unmeasured provider storage.
pub(super) fn observed_memory(
    phase: &'static str,
    owner: &'static str,
    bytes: crate::domain::recording::types::retention::value::RetainedBytes,
) {
    tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase, owner,
        objects = bytes.objects, metadata_capacity_bytes = bytes.metadata,
        packed_live_bytes = bytes.packed_live, packed_capacity_bytes = bytes.packed_capacity,
        staged_objects = bytes.staged_objects, staged_payload_bytes = bytes.staged_payload_bytes,
        text_live_bytes = bytes.text_live, text_capacity_bytes = bytes.text_capacity,
        notes_live_bytes = bytes.notes_live, notes_capacity_bytes = bytes.notes_capacity,
        findings_capacity_bytes = bytes.findings_capacity,
        decoded_live_bytes = bytes.cells_live, decoded_capacity_bytes = bytes.cells_capacity, decoded_rows = bytes.rows,
        places_live_bytes = bytes.places_live, places_capacity_bytes = bytes.places_capacity);
}

/// Record capacity traversal is disabled on the normal lint path.
pub(super) fn observed_records(phase: &'static str, records: &Vec<Record>) {
    if memory_enabled() {
        observed_memory(
            phase,
            "fresh_records",
            crate::domain::recording::types::retention::value::RetainedBytes::records(records),
        );
    }
}

/// The arena owner survives independently of the normalized text in fresh records.
pub(super) fn observed_arenas(
    phase: &'static str,
    arenas: &Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
) {
    if memory_enabled() {
        observed_memory(
            phase,
            "worker_arenas",
            crate::domain::recording::types::retention::value::RetainedBytes::arenas(arenas),
        );
    }
}

/// Cross-rule inputs and their name-place storage are observed without decoding/scanning any cells.
pub(super) fn observed_inputs(
    phase: &'static str,
    inputs: &Vec<crate::domain::table_model::types::Frontier>,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
) {
    if memory_enabled() {
        let mut bytes =
            crate::domain::recording::types::retention::value::RetainedBytes::inputs(inputs);
        let places = ryo_language::domain::symbols::service::places_mut(symbols);
        bytes.places_live =
            places.len() * size_of::<ryo_relations::domain::relations::types::NamePlace>();
        bytes.places_capacity =
            places.capacity() * size_of::<ryo_relations::domain::relations::types::NamePlace>();
        let text = ryo_language::domain::symbols::service::derived_of(symbols);
        bytes.text_live = text.0.len();
        bytes.text_capacity = text.0.capacity();
        observed_memory(phase, "decoded_inputs", bytes);
    }
}

/// Worker notes are a separate owner from their normalized record copy.
pub(super) fn observed_notes(
    phase: &'static str,
    notes: &crate::domain::workers::types::FreshNotes,
) {
    if memory_enabled() {
        observed_memory(
            phase,
            "worker_notes",
            crate::domain::recording::types::retention::value::RetainedBytes::worker_notes(notes),
        );
    }
}

/// Keep rule identity on decoder events without changing its domain API.
pub(super) fn memory_rule(
    id: &ryo_values::domain::runs::types::RuleId,
    index: usize,
) -> Option<tracing::span::EnteredSpan> {
    memory_enabled().then(|| tracing::trace_span!(target: "ryo::diagnostic::lifecycle_memory", "cross_rule_memory", rule = %id.0, rule_index = index).entered())
}

/// Correlate a bounded input-prepare interval with the compiled frontier identity.
pub(super) fn memory_input(
    input: ryo_values::domain::plan_parts::types::Input,
    index: usize,
) -> Option<tracing::span::EnteredSpan> {
    memory_enabled().then(|| tracing::trace_span!(target: "ryo::diagnostic::lifecycle_memory", "cross_input_memory", input_index = index, input = ?input).entered())
}

/// Keep provider bucket identity on temporary packed and decoded buffers.
pub(super) fn memory_bucket(bucket: Bucket) -> Option<tracing::span::EnteredSpan> {
    memory_enabled().then(|| tracing::trace_span!(target: "ryo::diagnostic::lifecycle_memory", "cross_bucket_memory", bucket = bucket.0).entered())
}

/// Do not walk stored payload metadata on ordinary lints.
pub(super) fn observed_stored(phase: &'static str, handed: &StoredHanded) {
    if memory_enabled() {
        observed_memory(
            phase,
            "store_handed",
            crate::domain::recording::types::retention::value::RetainedBytes::stored(handed),
        );
    }
}

/// Report the packed buffer only after the original provider read succeeded.
pub(super) fn observed_packed(words: &ryo_values::domain::stored_facts::types::KeptWords) {
    if memory_enabled() {
        observed_memory(
            "cross_input_packed",
            "packed_input",
            crate::domain::recording::types::retention::value::RetainedBytes::packed(words),
        );
    }
}

/// Publication observes staged descriptor capacity without reading spool contents.
pub(super) fn observed_staged(
    phase: &'static str,
    kept: &ryo_values::domain::stores::types::KeptRows,
) {
    if memory_enabled() {
        observed_memory(
            phase,
            "store_staged_descriptors",
            crate::domain::recording::types::retention::value::RetainedBytes::staged(kept),
        );
    }
}
