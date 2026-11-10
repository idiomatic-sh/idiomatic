//! The opt-in lifecycle memory diagnostics: component capacities written at the phases of a run, never walked when they are off.
use super::constants::{
    CROSS_SYMBOLS, FRESH_RECORDS, LIFECYCLE_MEMORY_DIAGNOSTIC, STORE_HANDED,
    STORE_STAGED_DESCRIPTORS, WORKER_ARENAS, WORKER_NOTES,
};
use super::types::{MemoryDiagnostics, MemoryOwner, MemoryPhase};
use ryo_values::domain::records::types::Record;
use ryo_values::domain::stores::types::StoredHanded;

/// One explicit opt-in gate; no capacity traversal or diagnostic span construction when disabled.
pub(crate) fn memory_diagnostics() -> MemoryDiagnostics {
    if tracing::enabled!(target: LIFECYCLE_MEMORY_DIAGNOSTIC, tracing::Level::TRACE) {
        MemoryDiagnostics::Enabled
    } else {
        MemoryDiagnostics::Disabled
    }
}

/// Component capacities are distinct from RSS, allocator retention and unmeasured provider storage.
pub(crate) fn observed_memory(
    phase: MemoryPhase,
    owner: MemoryOwner,
    bytes: ryo_values::domain::stores::types::retention::value::RetainedBytes,
) {
    tracing::trace!(target: LIFECYCLE_MEMORY_DIAGNOSTIC, phase = phase.0, owner = owner.0,
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
pub(crate) fn observed_records(phase: MemoryPhase, records: &Vec<Record>) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            FRESH_RECORDS,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::records(records),
        );
    }
}

/// The arena owner survives independently of the normalized text in fresh records.
pub(crate) fn observed_arenas(
    phase: MemoryPhase,
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
pub(crate) fn observed_cross_symbols(
    phase: MemoryPhase,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            CROSS_SYMBOLS,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::cross_symbols(
                symbols,
            ),
        );
    }
}

/// Worker notes are a separate owner from their normalized record copy.
pub(crate) fn observed_notes(
    phase: MemoryPhase,
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

/// Do not walk stored payload metadata on ordinary lints.
pub(crate) fn observed_stored(phase: MemoryPhase, handed: &StoredHanded) {
    if memory_diagnostics() == MemoryDiagnostics::Enabled {
        observed_memory(
            phase,
            STORE_HANDED,
            ryo_values::domain::stores::types::retention::value::RetainedBytes::stored(handed),
        );
    }
}

/// Publication observes staged descriptor capacity without reading spool contents.
pub(crate) fn observed_staged(
    phase: MemoryPhase,
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
