//! Exact capacities of the owned handover components at a lifecycle boundary.

use crate::domain::records::types::{FileOutcome, HandedNotes, Handing, Record, RecordWords};
use crate::domain::stored_facts::types::{ArenaBytes, KeptWords};
use crate::domain::stores::types::StoredHanded;

/// Separate owned components, never an RSS estimate or a claim to cover every allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetainedBytes {
    /// Number of records, arenas, frontiers or packed inputs represented.
    pub objects: usize,
    /// Capacity bytes of the measured owning vectors, excluding their nested payloads below.
    pub metadata: usize,
    /// Packed frontier bytes currently used.
    pub packed_live: usize,
    /// Packed frontier vector capacity bytes.
    pub packed_capacity: usize,
    /// Plain staged descriptors retained in memory, without their file-backed payload.
    pub staged_objects: usize,
    /// Logical bytes in the temporary spool, NOT resident memory.
    pub staged_payload_bytes: u64,
    /// Owned handover/arena text bytes currently used.
    pub text_live: usize,
    /// Owned handover/arena text vector capacity bytes.
    pub text_capacity: usize,
    /// Owned language-note bytes currently used.
    pub notes_live: usize,
    /// Owned language-note vector capacity bytes.
    pub notes_capacity: usize,
    /// Typed finding storage, its text, row ends and order, by actual capacity.
    pub findings_capacity: usize,
    /// Decoded cross-input cell bytes currently used.
    pub cells_live: usize,
    /// Decoded cross-input cell vector capacity bytes.
    pub cells_capacity: usize,
    /// Decoded rows currently owned.
    pub rows: usize,
    /// Cross-symbol name-place bytes currently used, supplied by the caller.
    pub places_live: usize,
    /// Cross-symbol name-place vector capacity bytes, supplied by the caller.
    pub places_capacity: usize,
}

impl RetainedBytes {
    /// Fresh records retain normalized packed handovers independently of the facts worker.
    pub fn records(records: &Vec<Record>) -> Self {
        let mut bytes = Self {
            objects: records.len(),
            metadata: capacity(records),
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
            bytes.metadata += capacity(&ran.frontier);
            bytes.findings_capacity += ran.rows.text.0.len()
                + capacity(&ran.rows.cells)
                + capacity(&ran.rows.ends.0)
                + capacity(&ran.rows.order);
            let Handing::Made(handed) = &ran.handed else {
                continue;
            };
            bytes.metadata += capacity(&handed.buckets)
                + capacity(&handed.governing.0)
                + capacity(&handed.partials);
            for bucket in &handed.buckets {
                match &bucket.words {
                    RecordWords::Inline(words) => {
                        bytes.packed_live += words.0.len();
                        bytes.packed_capacity += words.0.capacity();
                    }
                    RecordWords::Staged(span) => {
                        bytes.staged_objects += 1;
                        bytes.staged_payload_bytes =
                            bytes.staged_payload_bytes.saturating_add(span.length);
                    }
                }
            }
            bytes.text_live += handed.text.0.len();
            bytes.text_capacity += handed.text.0.capacity();
            if let HandedNotes::Noted(notes) = &handed.notes {
                bytes.notes_live += notes.notes.0.len();
                bytes.notes_capacity += notes.notes.0.capacity();
            }
        }
        bytes
    }

    /// The corresponding stored handover owns these same payloads after the records move.
    pub fn stored(handed: &StoredHanded) -> Self {
        let mut bytes = Self {
            objects: handed.frontier.len(),
            metadata: capacity(&handed.frontier)
                + capacity(&handed.texts)
                + capacity(&handed.notes)
                + capacity(&handed.covered),
            ..Self::default()
        };
        for bucket in &handed.frontier {
            bytes.packed_live += bucket.words.0.len();
            bytes.packed_capacity += bucket.words.0.capacity();
        }
        for text in &handed.texts {
            bytes.text_live += text.text.0.len();
            bytes.text_capacity += text.text.0.capacity();
        }
        for notes in &handed.notes {
            bytes.notes_live += notes.notes.notes.0.len();
            bytes.notes_capacity += notes.notes.notes.0.capacity();
        }
        for covered in &handed.covered {
            bytes.metadata += capacity(&covered.governing.0) + capacity(&covered.partials);
        }
        bytes
    }

    /// Store publication retains only descriptor metadata for fresh staged rows.
    pub fn staged(kept: &crate::domain::stores::types::KeptRows) -> Self {
        Self {
            objects: kept.staged.len(),
            metadata: capacity(&kept.staged),
            staged_objects: kept.staged.len(),
            staged_payload_bytes: kept
                .staged
                .iter()
                .map(|row| row.words.length)
                .fold(0_u64, u64::saturating_add),
            ..Self::default()
        }
    }

    /// Worker arenas are separate owners from normalized record handover text.
    pub fn arenas(arenas: &Vec<ArenaBytes>) -> Self {
        Self {
            objects: arenas.len(),
            metadata: capacity(arenas),
            text_live: arenas.iter().map(|arena| arena.0.len()).sum(),
            text_capacity: arenas.iter().map(|arena| arena.0.capacity()).sum(),
            ..Self::default()
        }
    }

    /// Original worker language notes may coexist with the copy normalized into fresh records.
    pub fn worker_notes<'a>(
        metadata: usize,
        notes: impl IntoIterator<Item = &'a crate::domain::stored_facts::types::FileNotes>,
    ) -> Self {
        let mut bytes = Self {
            metadata,
            ..Self::default()
        };
        for notes in notes {
            bytes.objects += 1;
            bytes.notes_live += notes.notes.0.len();
            bytes.notes_capacity += notes.notes.0.capacity();
        }
        bytes
    }

    /// A temporary caller-owned packed bucket; provider-internal storage is outside this measure.
    pub fn packed(words: &KeptWords) -> Self {
        Self {
            objects: 1,
            packed_live: words.0.0.len(),
            packed_capacity: words.0.0.capacity(),
            ..Self::default()
        }
    }

    /// Exact decoded frontier storage, independent of whether the cells are physically resident pages.
    pub fn inputs<'a>(metadata: usize, inputs: impl IntoIterator<Item = &'a Table>) -> Self {
        let mut bytes = Self {
            metadata,
            ..Self::default()
        };
        for table in inputs {
            bytes.objects += 1;
            bytes.metadata += capacity(&table.kinds);
            bytes.cells_live +=
                table.cells.len() * size_of::<ryo_relations::domain::relations::types::Value>();
            bytes.cells_capacity += capacity(&table.cells);
            bytes.rows += table.count.0;
        }
        bytes
    }

    /// One table while it is being decoded, before its final shrink.
    pub fn table(table: &Table) -> Self {
        Self {
            objects: 1,
            metadata: capacity(&table.kinds),
            cells_live: table.cells.len()
                * size_of::<ryo_relations::domain::relations::types::Value>(),
            cells_capacity: capacity(&table.cells),
            rows: table.count.0,
            ..Self::default()
        }
    }
}

/// Inline headers are already in their enclosing object; count only the allocation's payload here.
fn capacity<T>(values: &Vec<T>) -> usize {
    values.capacity().saturating_mul(size_of::<T>())
}

use crate::domain::rows::types::Table;
