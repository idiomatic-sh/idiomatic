//! Capacity snapshots neither decode, copy nor mutate the owned handover buffers.

use super::value::RetainedBytes;
use crate::domain::storing::service::stored_of;
use crate::domain::storing::types::ToKeep;
use crate::domain::table_model::types::{Frontier, RowCount, Table};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::relations::types::{CellKind, Integer, Relation, Value};
use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::records::types::{
    FileOutcome, FindingRows, Handed, HandedNotes, HandedWords, Handing, Ran, Record,
};
use ryo_values::domain::stored_facts::types::{
    ArenaBytes, Bucket, Digest, FileNotes, KeptWords, LanguageNumber, NoteBytes, PackedWords,
};
use ryo_values::domain::stores::types::{BucketsWanted, Stored};

fn bytes(capacity: usize, count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(capacity);
    bytes.resize(count, 1);
    bytes
}

#[test]
fn record_and_store_snapshots_follow_the_same_moved_payload_capacities() {
    let packed = bytes(37, 5);
    let packed_capacity = packed.capacity();
    let pointer = packed.as_ptr();
    let text = bytes(29, 3);
    let text_capacity = text.capacity();
    let notes = bytes(23, 4);
    let notes_capacity = notes.capacity();
    let mut records = Vec::with_capacity(3);
    records.push(Record {
        path: FilePath("case.py".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
        digest: Digest([0; 16]),
        number: FileNumber(0),
        outcome: FileOutcome::Read(Ran {
            rows: FindingRows::default(),
            number: FileNumber(0),
            interface: Digest([0; 16]),
            frontier: vec![],
            noted: Digest([0; 16]),
            handed: Handing::Made(Handed {
                buckets: vec![HandedWords {
                    bucket: Bucket(0),
                    words: PackedWords(packed),
                }],
                text: ArenaBytes(text),
                notes: HandedNotes::Noted(FileNotes {
                    language: LanguageNumber(0),
                    notes: NoteBytes(notes),
                }),
                ..Handed::default()
            }),
        }),
    });
    let before = RetainedBytes::records(&records);
    assert_eq!(
        (
            before.packed_live,
            before.packed_capacity,
            before.text_live,
            before.text_capacity,
            before.notes_live,
            before.notes_capacity
        ),
        (5, packed_capacity, 3, text_capacity, 4, notes_capacity)
    );
    assert!(before.metadata >= records.capacity() * size_of::<Record>());
    let stored = stored_of(
        ToKeep {
            rules: vec![],
            records,
            later: &FindingRows::default(),
            refused: &[],
            unknowns: &[],
            skipped: &[],
            consulted: &[],
            consulted_absent: &[],
        },
        &Stored::default(),
        &Default::default(),
        &BucketsWanted::All,
        vec![],
    );
    let after = RetainedBytes::stored(&stored.stored.handed);
    assert_eq!(
        (
            after.packed_live,
            after.packed_capacity,
            after.text_live,
            after.text_capacity,
            after.notes_live,
            after.notes_capacity
        ),
        (5, packed_capacity, 3, text_capacity, 4, notes_capacity)
    );
    assert_eq!(
        stored.stored.handed.frontier[0].words.0.as_ptr(),
        pointer,
        "observing and moving must not clone packed data"
    );
}

#[test]
fn decoded_and_arena_snapshots_count_capacity_without_touching_cells() {
    let mut cells = Vec::with_capacity(17);
    cells.extend([Value::Number(Integer(1)), Value::Number(Integer(2))]);
    let cell_capacity = cells.capacity();
    let pointer = cells.as_ptr();
    let mut kinds = Vec::with_capacity(5);
    kinds.extend([CellKind::Number, CellKind::Number]);
    let kinds_capacity = kinds.capacity();
    let mut inputs = Vec::with_capacity(3);
    inputs.push(Frontier {
        input: Input::Relation(Relation::Function),
        table: Table {
            cells,
            kinds,
            count: RowCount(1),
        },
    });
    let observed = RetainedBytes::inputs(&inputs);
    assert_eq!(
        (
            observed.objects,
            observed.rows,
            observed.cells_live,
            observed.cells_capacity
        ),
        (
            1,
            1,
            2 * size_of::<Value>(),
            cell_capacity * size_of::<Value>()
        )
    );
    assert_eq!(
        observed.metadata,
        inputs.capacity() * size_of::<Frontier>() + kinds_capacity * size_of::<CellKind>()
    );
    assert_eq!(inputs[0].table.cells.as_ptr(), pointer);
    let mut arenas = Vec::with_capacity(4);
    arenas.push(ArenaBytes(bytes(31, 7)));
    let observed = RetainedBytes::arenas(&arenas);
    assert_eq!(
        (observed.text_live, observed.text_capacity),
        (7, arenas[0].0.capacity())
    );
    assert_eq!(
        observed.metadata,
        arenas.capacity() * size_of::<ArenaBytes>()
    );
    let packed = KeptWords(PackedWords(bytes(19, 2)));
    assert_eq!(
        (
            RetainedBytes::packed(&packed).packed_live,
            RetainedBytes::packed(&packed).packed_capacity
        ),
        (2, packed.0.0.capacity())
    );
}
