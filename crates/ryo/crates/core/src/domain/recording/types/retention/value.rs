//! Exact capacities of the owned handover components at a lifecycle boundary.

use crate::domain::table_model::types::{Frontier, Table};
use ryo_values::domain::records::types::{FileOutcome, HandedNotes, Handing, Record};
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords};
use ryo_values::domain::stores::types::StoredHanded;

/// Separate owned components, never an RSS estimate or a claim to cover every allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RetainedBytes {
    /// Number of records, arenas, frontiers or packed inputs represented.
    pub objects: usize,
    /// Capacity bytes of the measured owning vectors, excluding their nested payloads below.
    pub metadata: usize,
    /// Packed frontier bytes currently used.
    pub packed_live: usize,
    /// Packed frontier vector capacity bytes.
    pub packed_capacity: usize,
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
    pub(crate) fn records(records: &Vec<Record>) -> Self {
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
                bytes.packed_live += bucket.words.0.len();
                bytes.packed_capacity += bucket.words.0.capacity();
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
    pub(crate) fn stored(handed: &StoredHanded) -> Self {
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

    /// Worker arenas are separate owners from normalized record handover text.
    pub(crate) fn arenas(arenas: &Vec<ArenaBytes>) -> Self {
        Self {
            objects: arenas.len(),
            metadata: capacity(arenas),
            text_live: arenas.iter().map(|arena| arena.0.len()).sum(),
            text_capacity: arenas.iter().map(|arena| arena.0.capacity()).sum(),
            ..Self::default()
        }
    }

    /// Original worker language notes may coexist with the copy normalized into fresh records.
    pub(crate) fn worker_notes(notes: &crate::domain::workers::types::FreshNotes) -> Self {
        Self {
            objects: notes.0.len(),
            metadata: capacity(&notes.0),
            notes_live: notes.0.iter().map(|entry| entry.notes.notes.0.len()).sum(),
            notes_capacity: notes
                .0
                .iter()
                .map(|entry| entry.notes.notes.0.capacity())
                .sum(),
            ..Self::default()
        }
    }

    /// A temporary caller-owned packed bucket; provider-internal storage is outside this measure.
    pub(crate) fn packed(words: &KeptWords) -> Self {
        Self {
            objects: 1,
            packed_live: words.0.0.len(),
            packed_capacity: words.0.0.capacity(),
            ..Self::default()
        }
    }

    /// Exact decoded frontier storage, independent of whether the cells are physically resident pages.
    pub(crate) fn inputs(inputs: &Vec<Frontier>) -> Self {
        let mut bytes = Self {
            objects: inputs.len(),
            metadata: capacity(inputs),
            ..Self::default()
        };
        for input in inputs {
            bytes.metadata += capacity(&input.table.kinds);
            bytes.cells_live += input.table.cells.len()
                * size_of::<ryo_language::domain::relations::types::Value>();
            bytes.cells_capacity += capacity(&input.table.cells);
            bytes.rows += input.table.count.0;
        }
        bytes
    }

    /// One table while it is being decoded, before its final shrink.
    pub(crate) fn table(table: &Table) -> Self {
        Self {
            objects: 1,
            metadata: capacity(&table.kinds),
            cells_live: table.cells.len()
                * size_of::<ryo_language::domain::relations::types::Value>(),
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
