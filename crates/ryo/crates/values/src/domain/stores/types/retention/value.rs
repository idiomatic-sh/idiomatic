//! Exact capacities of the owned handover components at a lifecycle boundary.

use crate::domain::records::types::{FileOutcome, HandedNotes, Handing, Record, RecordWords};
use crate::domain::stored_facts::types::{ArenaBytes, KeptWords};
use crate::domain::stores::types::StoredHanded;

/// Separate owned components, never an RSS estimate or a claim to cover every allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetainedBytes {
    /// Number of records, arenas, frontiers or packed inputs represented.
    pub objects: crate::domain::stores::types::ObjectCount,
    /// Capacity bytes of the measured owning vectors, excluding their nested payloads below.
    pub metadata: crate::domain::stores::types::CapacityBytes,
    /// Packed frontier bytes currently used.
    pub packed_live: crate::domain::stores::types::LiveBytes,
    /// Packed frontier vector capacity bytes.
    pub packed_capacity: crate::domain::stores::types::CapacityBytes,
    /// Plain staged descriptors retained in memory, without their file-backed payload.
    pub staged_objects: crate::domain::stores::types::ObjectCount,
    /// Logical bytes in the temporary spool, NOT resident memory.
    pub staged_payload_bytes: crate::domain::records::types::PayloadLength,
    /// Owned handover/arena text bytes currently used.
    pub text_live: crate::domain::stores::types::LiveBytes,
    /// Owned handover/arena text vector capacity bytes.
    pub text_capacity: crate::domain::stores::types::CapacityBytes,
    /// Owned language-note bytes currently used.
    pub notes_live: crate::domain::stores::types::LiveBytes,
    /// Owned language-note vector capacity bytes.
    pub notes_capacity: crate::domain::stores::types::CapacityBytes,
    /// Typed finding storage, its text, row ends and order, by actual capacity.
    pub findings_capacity: crate::domain::stores::types::CapacityBytes,
    /// Decoded cross-input cell bytes currently used.
    pub cells_live: crate::domain::stores::types::LiveBytes,
    /// Decoded cross-input cell vector capacity bytes.
    pub cells_capacity: crate::domain::stores::types::CapacityBytes,
    /// Decoded rows currently owned.
    pub rows: crate::domain::rows::types::RowCount,
    /// Cross-symbol name-place bytes currently used, supplied by the caller.
    pub places_live: crate::domain::stores::types::LiveBytes,
    /// Cross-symbol name-place vector capacity bytes, supplied by the caller.
    pub places_capacity: crate::domain::stores::types::CapacityBytes,
}

impl RetainedBytes {
    /// Fresh records retain normalized packed handovers independently of the facts worker.
    pub fn records(records: &Vec<Record>) -> Self {
        let mut bytes = Self {
            objects: crate::domain::stores::types::ObjectCount(records.len()),
            metadata: crate::domain::stores::types::CapacityBytes(capacity(records).0),
            ..Self::default()
        };
        for record in records {
            let ran = match &record.outcome {
                FileOutcome::Read(ran) => Some(ran),
                FileOutcome::Unread { ran, .. } => ran.as_ref(),
            };
            let Some(ran) = ran else {
                continue;
            };
            bytes.metadata.0 += capacity(&ran.frontier).0;
            bytes.findings_capacity.0 += ran.rows.text.0.len()
                + capacity(&ran.rows.cells).0
                + capacity(&ran.rows.ends.0).0
                + capacity(&ran.rows.order).0;
            let Handing::Made(handed) = &ran.handed else {
                continue;
            };
            bytes.metadata.0 += capacity(&handed.buckets).0
                + capacity(&handed.governing.0).0
                + capacity(&handed.partials).0;
            for bucket in &handed.buckets {
                match &bucket.handed {
                    RecordWords::Inline(words) => {
                        bytes.packed_live.0 += words.0.len();
                        bytes.packed_capacity.0 += words.0.capacity();
                    }
                    RecordWords::Staged(span) => {
                        bytes.staged_objects.0 += 1;
                        bytes.staged_payload_bytes.0 =
                            bytes.staged_payload_bytes.0.saturating_add(span.length.0);
                    }
                }
            }
            bytes.text_live.0 += handed.text.0.len();
            bytes.text_capacity.0 += handed.text.0.capacity();
            if let HandedNotes::Noted(notes) = &handed.notes {
                bytes.notes_live.0 += notes.notes.0.len();
                bytes.notes_capacity.0 += notes.notes.0.capacity();
            }
        }
        bytes
    }

    /// The corresponding stored handover owns these same payloads after the records move.
    pub fn stored(handed: &StoredHanded) -> Self {
        let mut bytes = Self {
            objects: crate::domain::stores::types::ObjectCount(handed.frontier.len()),
            metadata: crate::domain::stores::types::CapacityBytes(
                capacity(&handed.frontier).0
                    + capacity(&handed.texts).0
                    + capacity(&handed.notes).0
                    + capacity(&handed.covered).0,
            ),
            ..Self::default()
        };
        for bucket in &handed.frontier {
            bytes.packed_live.0 += bucket.words.0.len();
            bytes.packed_capacity.0 += bucket.words.0.capacity();
        }
        for text in &handed.texts {
            bytes.text_live.0 += text.text.0.len();
            bytes.text_capacity.0 += text.text.0.capacity();
        }
        for notes in &handed.notes {
            bytes.notes_live.0 += notes.notes.notes.0.len();
            bytes.notes_capacity.0 += notes.notes.notes.0.capacity();
        }
        for covered in &handed.covered {
            bytes.metadata.0 += capacity(&covered.governing.0).0 + capacity(&covered.partials).0;
        }
        bytes
    }

    /// Store publication retains only descriptor metadata for fresh staged rows.
    pub fn staged(kept: &crate::domain::stores::types::KeptRows) -> Self {
        Self {
            objects: crate::domain::stores::types::ObjectCount(kept.staged.len()),
            metadata: crate::domain::stores::types::CapacityBytes(capacity(&kept.staged).0),
            staged_objects: crate::domain::stores::types::ObjectCount(kept.staged.len()),
            staged_payload_bytes: crate::domain::records::types::PayloadLength(
                kept.staged
                    .iter()
                    .map(|row| row.words.length.0)
                    .fold(0_u64, u64::saturating_add),
            ),
            ..Self::default()
        }
    }

    /// Worker arenas are separate owners from normalized record handover text.
    pub fn arenas(arenas: &Vec<ArenaBytes>) -> Self {
        Self {
            objects: crate::domain::stores::types::ObjectCount(arenas.len()),
            metadata: crate::domain::stores::types::CapacityBytes(capacity(arenas).0),
            text_live: crate::domain::stores::types::LiveBytes(
                arenas.iter().map(|arena| arena.0.len()).sum(),
            ),
            text_capacity: crate::domain::stores::types::CapacityBytes(
                arenas.iter().map(|arena| arena.0.capacity()).sum(),
            ),
            ..Self::default()
        }
    }

    /// Original worker notes lend their actual owner and the note field of each entry.
    /// Metadata is derived from the reserved allocation of the vector that holds them, including unused capacity.
    ///
    /// ```compile_fail
    /// use ryo_values::domain::records::types::retention::value::RetainedBytes;
    /// use ryo_values::domain::stored_facts::types::FileNotes;
    /// let notes: Vec<FileNotes> = Vec::new();
    /// let _ = RetainedBytes::worker_notes(0, notes.iter());
    /// ```
    pub fn worker_notes<T>(
        owner: &Vec<T>,
        notes: impl Fn(&T) -> &crate::domain::stored_facts::types::FileNotes,
    ) -> Self {
        let mut bytes = Self {
            objects: crate::domain::stores::types::ObjectCount(owner.len()),
            metadata: crate::domain::stores::types::CapacityBytes(capacity(owner).0),
            ..Self::default()
        };
        for entry in owner {
            let notes = notes(entry);
            bytes.notes_live.0 += notes.notes.0.len();
            bytes.notes_capacity.0 += notes.notes.0.capacity();
        }
        bytes
    }

    /// The places and the derived text that the symbols of the cross-file steps hold.
    pub fn cross_symbols(symbols: &mut ryo_language::domain::symbols::types::Symbols) -> Self {
        let mut bytes = Self::default();
        let places = ryo_language::domain::symbols::service::places_mut(symbols);
        bytes.places_live.0 =
            places.len() * size_of::<ryo_relations::domain::relations::types::NamePlace>();
        bytes.places_capacity.0 =
            places.capacity() * size_of::<ryo_relations::domain::relations::types::NamePlace>();
        let text = ryo_language::domain::names::service::derived_of(symbols);
        bytes.text_live.0 = text.0.len();
        bytes.text_capacity.0 = text.0.capacity();
        bytes
    }

    /// A temporary caller-owned packed bucket; provider-internal storage is outside this measure.
    pub fn packed(words: &KeptWords) -> Self {
        Self {
            objects: crate::domain::stores::types::ObjectCount(1),
            packed_live: crate::domain::stores::types::LiveBytes(words.0.0.len()),
            packed_capacity: crate::domain::stores::types::CapacityBytes(words.0.0.capacity()),
            ..Self::default()
        }
    }

    /// Exact decoded inputs borrow the actual owning vector and each entry's table field.
    /// The owning allocation is measured here rather than supplied as a caller-computed byte count.
    ///
    /// ```compile_fail
    /// use ryo_values::domain::records::types::retention::value::RetainedBytes;
    /// use ryo_values::domain::rows::types::Table;
    /// let tables: Vec<Table> = Vec::new();
    /// let _ = RetainedBytes::inputs(0, tables.iter());
    /// ```
    pub fn inputs<T>(owner: &Vec<T>, table: impl Fn(&T) -> &Table) -> Self {
        let mut bytes = Self {
            objects: crate::domain::stores::types::ObjectCount(owner.len()),
            metadata: crate::domain::stores::types::CapacityBytes(capacity(owner).0),
            ..Self::default()
        };
        for entry in owner {
            let table = table(entry);
            bytes.metadata.0 += capacity(&table.kinds).0;
            bytes.cells_live.0 +=
                table.cells.len() * size_of::<ryo_relations::domain::relations::types::Value>();
            bytes.cells_capacity.0 += capacity(&table.cells).0;
            bytes.rows.0 += table.count.0;
        }
        bytes
    }

    /// One table while it is being decoded, before its final shrink.
    pub fn table(table: &Table) -> Self {
        Self {
            objects: crate::domain::stores::types::ObjectCount(1),
            metadata: crate::domain::stores::types::CapacityBytes(capacity(&table.kinds).0),
            cells_live: crate::domain::stores::types::LiveBytes(
                table.cells.len() * size_of::<ryo_relations::domain::relations::types::Value>(),
            ),
            cells_capacity: crate::domain::stores::types::CapacityBytes(capacity(&table.cells).0),
            rows: crate::domain::rows::types::RowCount(table.count.0),
            ..Self::default()
        }
    }
}

/// Inline headers are already in their enclosing object; count only the allocation's payload here.
fn capacity<T>(values: &Vec<T>) -> crate::domain::stores::types::CapacityBytes {
    crate::domain::stores::types::CapacityBytes(values.capacity().saturating_mul(size_of::<T>()))
}

use crate::domain::rows::types::Table;
