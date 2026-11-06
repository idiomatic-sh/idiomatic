//! Capacity snapshots neither decode, copy nor mutate the owned handover buffers.

use crate::domain::storing::service::stored_of;
use crate::domain::storing::types::ToKeep;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::records::types::{
    FileOutcome, FindingRows, Handed, HandedNotes, HandedWords, Handing, Ran, Record,
};
use ryo_values::domain::stored_facts::types::{
    ArenaBytes, Bucket, Digest, FileNotes, KeptWords, LanguageNumber, NoteBytes, PackedWords,
};
use ryo_values::domain::stores::types::retention::value::RetainedBytes;
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, Stored};

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
        executable: Default::default(),
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
                    proof: crate::outbound::lint_store::fixtures::proof(),
                    words: PackedWords(packed).into(),
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
            project_observations: &[],
        },
        &Stored::default(),
        &ReadHanded::default(),
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
    inputs.push(Table {
        cells,
        kinds,
        count: RowCount(1),
    });
    let observed = RetainedBytes::inputs(&inputs, |input| input);
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
        inputs.capacity() * size_of::<Table>() + kinds_capacity * size_of::<CellKind>()
    );
    assert_eq!(inputs[0].cells.as_ptr(), pointer);
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

#[test]
fn staged_records_and_final_store_retain_only_descriptors_for_large_logical_payloads() {
    use ryo_values::domain::records::types::{RecordWords, StagedWords, StagingSession};
    let mut records: Vec<_> = (0..128)
        .map(|number| Record {
            executable: Default::default(),
            path: FilePath(format!("{number}.py").into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
            digest: Digest([0; 16]),
            number: FileNumber(number),
            outcome: FileOutcome::Read(Ran {
                rows: FindingRows::default(),
                number: FileNumber(number),
                interface: Digest([0; 16]),
                frontier: vec![],
                noted: Digest([0; 16]),
                handed: Handing::Made(Handed {
                    buckets: vec![HandedWords {
                        bucket: Bucket(0),
                        proof: crate::outbound::lint_store::fixtures::proof(),
                        words: RecordWords::Staged(StagedWords {
                            session: StagingSession(1),
                            offset: ryo_values::domain::records::types::PayloadOffset(u64::from(number) * 1024 * 1024),
                            length: ryo_values::domain::records::types::PayloadLength(1024 * 1024),
                        }),
                    }],
                    ..Handed::default()
                }),
            }),
        })
        .collect();
    let before = RetainedBytes::records(&records);
    assert_eq!(before.packed_capacity, 0);
    assert_eq!(before.staged_objects, 128);
    assert_eq!(before.staged_payload_bytes, 128 * 1024 * 1024);
    let mut store = ryo_values::domain::stores::types::StoredHanded::default();
    let mut staged = Vec::new();
    for record in records.drain(..) {
        let FileOutcome::Read(ran) = record.outcome else {
            panic!("read")
        };
        let Handing::Made(handed) = ran.handed else {
            panic!("made")
        };
        crate::domain::handing::service::hand_into(
            &mut store,
            ryo_values::domain::stores::types::StoredFileNumber(record.number.0),
            handed,
            &BucketsWanted::All,
            &mut staged,
        );
    }
    assert!(store.frontier.is_empty());
    let kept = ryo_values::domain::stores::types::KeptRows {
        staged,
        ..Default::default()
    };
    let after = RetainedBytes::staged(&kept);
    assert_eq!(after.packed_capacity, 0);
    assert_eq!(after.staged_objects, 128);
    assert_eq!(after.staged_payload_bytes, before.staged_payload_bytes);
    assert!(
        after.metadata < 128 * 1024,
        "metadata stays proportional to bucket count, not payload size"
    );
}

use ryo_values::domain::rows::types::{RowCount, Table};
