//! Capacity snapshots neither decode, copy nor mutate the owned handover buffers: what the records and the store retain.

use crate::domain::storing::service::stored_of;
use crate::domain::storing::types::ToKeep;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::files::types::{FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::records::types::{
    FileOutcome, Handed, HandedNotes, HandedWords, Handing, Ran, Record,
};
use ryo_values::domain::row_texts::types::FindingRows;
use ryo_values::domain::stored_facts::types::{
    ArenaBytes, Bucket, Digest, FileNotes, KeptWords, LanguageNumber, NoteBytes, PackedWords,
};
use ryo_values::domain::stores::types::retention::value::RetainedBytes;
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, Stored};
use ryo_values::domain::rows::types::{RowCount, Table};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::convert::Infallible;

/// A scenario: what a row asks of the part.
type Input = fn() -> Value;
/// What a scenario answers: a yes as one.
type Value = u8;
/// Nothing of this part is refused.
type Failure = Infallible;

fn bytes(capacity: usize, count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(capacity);
    bytes.resize(count, 1);
    bytes
}

fn record_and_store_snapshots_follow_the_same_moved_payload_capacities() -> Value {
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
            before.packed_live.0,
            before.packed_capacity.0,
            before.text_live.0,
            before.text_capacity.0,
            before.notes_live.0,
            before.notes_capacity.0
        ),
        (5, packed_capacity, 3, text_capacity, 4, notes_capacity)
    );
    assert!(before.metadata.0 >= records.capacity() * size_of::<Record>());
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
            after.packed_live.0,
            after.packed_capacity.0,
            after.text_live.0,
            after.text_capacity.0,
            after.notes_live.0,
            after.notes_capacity.0
        ),
        (5, packed_capacity, 3, text_capacity, 4, notes_capacity)
    );
    assert_eq!(
        stored.stored.handed.frontier[0].words.0.as_ptr(),
        pointer,
        "observing and moving must not clone packed data"
    );
    1
}

fn decoded_and_arena_snapshots_count_capacity_without_touching_cells() -> Value {
    let mut cells = Vec::with_capacity(17);
    cells.extend([Cell::Number(Integer(1)), Cell::Number(Integer(2))]);
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
            observed.objects.0,
            observed.rows.0,
            observed.cells_live.0,
            observed.cells_capacity.0
        ),
        (
            1,
            1,
            2 * size_of::<Cell>(),
            cell_capacity * size_of::<Cell>()
        )
    );
    assert_eq!(
        observed.metadata.0,
        inputs.capacity() * size_of::<Table>() + kinds_capacity * size_of::<CellKind>()
    );
    assert_eq!(inputs[0].cells.as_ptr(), pointer);
    let mut arenas = Vec::with_capacity(4);
    arenas.push(ArenaBytes(bytes(31, 7)));
    let observed = RetainedBytes::arenas(&arenas);
    assert_eq!(
        (observed.text_live.0, observed.text_capacity.0),
        (7, arenas[0].0.capacity())
    );
    assert_eq!(
        observed.metadata.0,
        arenas.capacity() * size_of::<ArenaBytes>()
    );
    let packed = KeptWords(PackedWords(bytes(19, 2)));
    assert_eq!(
        (
            RetainedBytes::packed(&packed).packed_live,
            RetainedBytes::packed(&packed).packed_capacity
        ),
        (
            ryo_values::domain::stores::types::LiveBytes(2),
            ryo_values::domain::stores::types::CapacityBytes(packed.0.0.capacity())
        )
    );
    1
}

fn staged_records_and_final_store_retain_only_descriptors_for_large_logical_payloads() -> Value {
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
                            offset: ryo_values::domain::records::types::PayloadOffset(
                                u64::from(number) * 1024 * 1024,
                            ),
                            length: ryo_values::domain::records::types::PayloadLength(1024 * 1024),
                        }),
                    }],
                    ..Handed::default()
                }),
            }),
        })
        .collect();
    let before = RetainedBytes::records(&records);
    assert_eq!(before.packed_capacity.0, 0);
    assert_eq!(before.staged_objects.0, 128);
    assert_eq!(before.staged_payload_bytes.0, 128 * 1024 * 1024);
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
            ryo_values::domain::stored_findings::types::StoredFileNumber(record.number.0),
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
    assert_eq!(after.packed_capacity.0, 0);
    assert_eq!(after.staged_objects.0, 128);
    assert_eq!(after.staged_payload_bytes.0, before.staged_payload_bytes.0);
    assert!(
        after.metadata.0 < 128 * 1024,
        "metadata stays proportional to bucket count, not payload size"
    );
    1
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("record_and_store_snapshots_follow_the_same_moved_payload_capacities"),
        word: Word::Succeeds,
        input: || record_and_store_snapshots_follow_the_same_moved_payload_capacities,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("decoded_and_arena_snapshots_count_capacity_without_touching_cells"),
        word: Word::Succeeds,
        input: || decoded_and_arena_snapshots_count_capacity_without_touching_cells,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("staged_records_and_final_store_retain_only_descriptors_for_large_logical_payloads"),
        word: Word::Succeeds,
        input: || staged_records_and_final_store_retain_only_descriptors_for_large_logical_payloads,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Value {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |scenario| {
        Ok::<_, Failure>(subject(scenario))
    });
}
