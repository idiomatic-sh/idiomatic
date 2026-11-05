//! Owning-container capacities, live payloads and descriptor-only storage have independent exact expectations.
use super::value::RetainedBytes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    files::types::{FileNumber, FileSize, Nanoseconds},
    records::types::{
        FileOutcome, FindingRows, FrontierCoverage, FrontierProof, GoverningRules, Handed,
        HandedNotes, HandedWords, Handing, Ran, Record, RecordWords, RuleFrontier, StagedWords,
        StagingSession,
    },
    rows::types::{RowCount, Table},
    stored_facts::types::{
        ArenaBytes, Bucket, Digest, FileNotes, KeptWords, LanguageNumber, NoteBytes, PackedWords,
        StoredPartial,
    },
    stores::types::{
        KeptRows, StagedRow, StoredCovered, StoredFileNumber, StoredHanded, StoredNotes,
        StoredText, StoredWords,
    },
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
#[derive(Clone, Copy)]
enum Scenario {
    Inputs,
    WorkerNotes,
    ReservedEmpty,
    Payloads,
    Records,
    Stored,
    Staged,
}
type Input = Scenario;
type Value = [u64; 4];
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("input_metadata_comes_from_actual_owner_capacity_and_embedded_tables"),
        word: Word::Succeeds,
        input: || Scenario::Inputs,
        expect: Expect::Value([2, 3, 4 * size_of::<Cell>() as u64, 0]),
    },
    Case {
        name: CaseName("worker_note_metadata_uses_owner_layout_and_preserves_note_capacities"),
        word: Word::Succeeds,
        input: || Scenario::WorkerNotes,
        expect: Expect::Value([2, 0, 7, 0]),
    },
    Case {
        name: CaseName("empty_owners_still_report_reserved_metadata_allocations"),
        word: Word::Succeeds,
        input: || Scenario::ReservedEmpty,
        expect: Expect::Value([0, 0, 0, 0]),
    },
    Case {
        name: CaseName("packed_arena_and_table_payloads_keep_live_and_capacity_separate"),
        word: Word::Succeeds,
        input: || Scenario::Payloads,
        expect: Expect::Value([1, 2, 4 * size_of::<Cell>() as u64, 0]),
    },
    Case {
        name: CaseName("record_metadata_includes_reserved_nested_headers_without_copying_payloads"),
        word: Word::Succeeds,
        input: || Scenario::Records,
        expect: Expect::Value([1, 0, 12, 11]),
    },
    Case {
        name: CaseName("stored_payload_metadata_counts_the_actual_owning_vectors"),
        word: Word::Succeeds,
        input: || Scenario::Stored,
        expect: Expect::Value([1, 0, 12, 0]),
    },
    Case {
        name: CaseName("staged_measurement_counts_descriptors_and_saturates_logical_bytes"),
        word: Word::Succeeds,
        input: || Scenario::Staged,
        expect: Expect::Value([2, 0, 0, u64::MAX]),
    },
];
struct InputOwner {
    tag: [u64; 3],
    table: Table,
}
struct NoteOwner {
    tag: [u64; 3],
    notes: FileNotes,
}
fn bytes(capacity: usize, live: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(capacity);
    bytes.resize(live, 1);
    bytes
}
fn table(kind_capacity: usize, cell_capacity: usize, width: usize, rows: usize) -> Table {
    let mut kinds = Vec::with_capacity(kind_capacity);
    kinds.resize(width, CellKind::Number);
    let mut cells = Vec::with_capacity(cell_capacity);
    cells.resize(width * rows, Cell::Number(Integer(7)));
    Table {
        kinds,
        cells,
        count: RowCount(rows),
    }
}
fn notes(capacity: usize, live: usize) -> FileNotes {
    FileNotes {
        language: LanguageNumber(0),
        notes: NoteBytes(bytes(capacity, live)),
    }
}
fn proof() -> FrontierProof {
    FrontierProof {
        version: 1,
        rule: 0,
        input: 0,
        rule_digest: Digest([0; 16]),
        coverage: FrontierCoverage::Complete,
    }
}
fn observed(bytes: RetainedBytes) -> Value {
    [
        u64::try_from(bytes.objects).unwrap(),
        u64::try_from(bytes.rows).unwrap(),
        u64::try_from(bytes.cells_live + bytes.packed_live + bytes.text_live + bytes.notes_live)
            .unwrap(),
        bytes.staged_payload_bytes,
    ]
}
fn inputs() -> RetainedBytes {
    let mut owner = Vec::with_capacity(5);
    owner.push(InputOwner {
        tag: [1, 2, 3],
        table: table(7, 11, 2, 1),
    });
    owner.push(InputOwner {
        tag: [4, 5, 6],
        table: table(9, 19, 1, 2),
    });
    let (owner_pointer, first_cells, second_cells) = (
        owner.as_ptr(),
        owner[0].table.cells.as_ptr(),
        owner[1].table.cells.as_ptr(),
    );
    let actual = RetainedBytes::inputs(&owner, |entry| &entry.table);
    assert_eq!(
        actual,
        RetainedBytes {
            objects: 2,
            metadata: owner.capacity() * size_of::<InputOwner>()
                + (owner[0].table.kinds.capacity() + owner[1].table.kinds.capacity())
                    * size_of::<CellKind>(),
            rows: 3,
            cells_live: 4 * size_of::<Cell>(),
            cells_capacity: (owner[0].table.cells.capacity() + owner[1].table.cells.capacity())
                * size_of::<Cell>(),
            ..RetainedBytes::default()
        }
    );
    assert!(owner.capacity() > owner.len());
    assert!(size_of::<InputOwner>() > size_of::<Table>());
    assert_eq!(
        (
            owner.as_ptr(),
            owner[0].table.cells.as_ptr(),
            owner[1].table.cells.as_ptr()
        ),
        (owner_pointer, first_cells, second_cells)
    );
    assert_eq!((owner[0].tag, owner[1].tag), ([1, 2, 3], [4, 5, 6]));
    actual
}
fn worker_notes() -> RetainedBytes {
    let mut owner = Vec::with_capacity(6);
    owner.push(NoteOwner {
        tag: [1, 2, 3],
        notes: notes(29, 4),
    });
    owner.push(NoteOwner {
        tag: [4, 5, 6],
        notes: notes(17, 3),
    });
    let pointers = (
        owner.as_ptr(),
        owner[0].notes.notes.0.as_ptr(),
        owner[1].notes.notes.0.as_ptr(),
    );
    let actual = RetainedBytes::worker_notes(&owner, |entry| &entry.notes);
    assert_eq!(
        actual,
        RetainedBytes {
            objects: 2,
            metadata: owner.capacity() * size_of::<NoteOwner>(),
            notes_live: 7,
            notes_capacity: owner[0].notes.notes.0.capacity() + owner[1].notes.notes.0.capacity(),
            ..RetainedBytes::default()
        }
    );
    assert!(owner.capacity() > owner.len());
    assert!(size_of::<NoteOwner>() > size_of::<FileNotes>());
    assert_eq!(
        (
            owner.as_ptr(),
            owner[0].notes.notes.0.as_ptr(),
            owner[1].notes.notes.0.as_ptr()
        ),
        pointers
    );
    assert_eq!((owner[0].tag, owner[1].tag), ([1, 2, 3], [4, 5, 6]));
    actual
}
fn reserved_empty() -> RetainedBytes {
    let inputs = Vec::<InputOwner>::with_capacity(4);
    let notes = Vec::<NoteOwner>::with_capacity(3);
    let actual = RetainedBytes::inputs(&inputs, |entry| &entry.table);
    let noted = RetainedBytes::worker_notes(&notes, |entry| &entry.notes);
    assert_eq!(
        actual,
        RetainedBytes {
            metadata: inputs.capacity() * size_of::<InputOwner>(),
            ..RetainedBytes::default()
        }
    );
    assert_eq!(
        noted,
        RetainedBytes {
            metadata: notes.capacity() * size_of::<NoteOwner>(),
            ..RetainedBytes::default()
        }
    );
    actual
}
fn payloads() -> RetainedBytes {
    let mut arenas = Vec::with_capacity(3);
    arenas.push(ArenaBytes(bytes(17, 3)));
    arenas.push(ArenaBytes(bytes(19, 4)));
    assert_eq!(
        RetainedBytes::arenas(&arenas),
        RetainedBytes {
            objects: 2,
            metadata: arenas.capacity() * size_of::<ArenaBytes>(),
            text_live: 7,
            text_capacity: arenas[0].0.capacity() + arenas[1].0.capacity(),
            ..RetainedBytes::default()
        }
    );
    let packed = KeptWords(PackedWords(bytes(23, 5)));
    let pointer = packed.0.0.as_ptr();
    assert_eq!(
        RetainedBytes::packed(&packed),
        RetainedBytes {
            objects: 1,
            packed_live: 5,
            packed_capacity: packed.0.0.capacity(),
            ..RetainedBytes::default()
        }
    );
    assert_eq!(packed.0.0.as_ptr(), pointer);
    let table = table(5, 11, 2, 2);
    let actual = RetainedBytes::table(&table);
    assert_eq!(
        actual,
        RetainedBytes {
            objects: 1,
            metadata: table.kinds.capacity() * size_of::<CellKind>(),
            rows: 2,
            cells_live: 4 * size_of::<Cell>(),
            cells_capacity: table.cells.capacity() * size_of::<Cell>(),
            ..RetainedBytes::default()
        }
    );
    actual
}
fn records() -> RetainedBytes {
    let packed = bytes(37, 5);
    let (packed_capacity, packed_pointer) = (packed.capacity(), packed.as_ptr());
    let text = bytes(29, 3);
    let text_capacity = text.capacity();
    let notes = notes(23, 4);
    let notes_capacity = notes.notes.0.capacity();
    let mut buckets = Vec::with_capacity(4);
    buckets.push(HandedWords {
        bucket: Bucket(0),
        proof: proof(),
        words: RecordWords::Inline(PackedWords(packed)),
    });
    buckets.push(HandedWords {
        bucket: Bucket(1),
        proof: proof(),
        words: RecordWords::Staged(StagedWords {
            session: StagingSession(1),
            offset: 0,
            length: 11,
        }),
    });
    let frontier = Vec::<RuleFrontier>::with_capacity(2);
    let governing = Vec::<u32>::with_capacity(5);
    let partials = Vec::<StoredPartial>::with_capacity(6);
    let nested_metadata = buckets.capacity() * size_of::<HandedWords>()
        + frontier.capacity() * size_of::<RuleFrontier>()
        + governing.capacity() * size_of::<u32>()
        + partials.capacity() * size_of::<StoredPartial>();
    let mut owner = Vec::with_capacity(3);
    owner.push(Record {
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
            frontier,
            noted: Digest([0; 16]),
            handed: Handing::Made(Handed {
                buckets,
                text: ArenaBytes(text),
                notes: HandedNotes::Noted(notes),
                governing: GoverningRules(governing),
                partials,
                ..Handed::default()
            }),
        }),
    });
    let actual = RetainedBytes::records(&owner);
    assert_eq!(
        actual,
        RetainedBytes {
            objects: 1,
            metadata: owner.capacity() * size_of::<Record>() + nested_metadata,
            packed_live: 5,
            packed_capacity,
            staged_objects: 1,
            staged_payload_bytes: 11,
            text_live: 3,
            text_capacity,
            notes_live: 4,
            notes_capacity,
            ..RetainedBytes::default()
        }
    );
    let FileOutcome::Read(ran) = &owner[0].outcome else {
        panic!("read fixture")
    };
    let Handing::Made(handed) = &ran.handed else {
        panic!("fresh handover")
    };
    let RecordWords::Inline(words) = &handed.buckets[0].words else {
        panic!("inline first bucket")
    };
    assert_eq!(words.0.as_ptr(), packed_pointer);
    actual
}
fn stored() -> RetainedBytes {
    let mut owner = StoredHanded {
        frontier: Vec::with_capacity(3),
        texts: Vec::with_capacity(4),
        notes: Vec::with_capacity(5),
        covered: Vec::with_capacity(6),
        ..StoredHanded::default()
    };
    owner.frontier.push(StoredWords {
        file: StoredFileNumber(0),
        bucket: Bucket(0),
        proof: proof(),
        words: PackedWords(bytes(37, 5)),
    });
    owner.texts.push(StoredText {
        file: StoredFileNumber(0),
        text: ArenaBytes(bytes(29, 3)),
    });
    owner.notes.push(StoredNotes {
        file: StoredFileNumber(0),
        notes: notes(23, 4),
    });
    owner.covered.push(StoredCovered {
        file: StoredFileNumber(0),
        governing: GoverningRules(Vec::with_capacity(7)),
        partials: Vec::with_capacity(8),
    });
    let actual = RetainedBytes::stored(&owner);
    assert_eq!(
        actual,
        RetainedBytes {
            objects: 1,
            metadata: owner.frontier.capacity() * size_of::<StoredWords>()
                + owner.texts.capacity() * size_of::<StoredText>()
                + owner.notes.capacity() * size_of::<StoredNotes>()
                + owner.covered.capacity() * size_of::<StoredCovered>()
                + owner.covered[0].governing.0.capacity() * size_of::<u32>()
                + owner.covered[0].partials.capacity() * size_of::<StoredPartial>(),
            packed_live: 5,
            packed_capacity: owner.frontier[0].words.0.capacity(),
            text_live: 3,
            text_capacity: owner.texts[0].text.0.capacity(),
            notes_live: 4,
            notes_capacity: owner.notes[0].notes.notes.0.capacity(),
            ..RetainedBytes::default()
        }
    );
    actual
}
fn staged() -> RetainedBytes {
    let mut rows = Vec::with_capacity(3);
    for (ordinal, length) in [(0, u64::MAX - 5), (1, 9)] {
        rows.push(StagedRow {
            file: StoredFileNumber(0),
            bucket: Bucket(0),
            ordinal,
            words: StagedWords {
                session: StagingSession(1),
                offset: 0,
                length,
            },
        });
    }
    let owner = KeptRows {
        staged: rows,
        ..KeptRows::default()
    };
    let actual = RetainedBytes::staged(&owner);
    assert_eq!(
        actual,
        RetainedBytes {
            objects: 2,
            metadata: owner.staged.capacity() * size_of::<StagedRow>(),
            staged_objects: 2,
            staged_payload_bytes: u64::MAX,
            ..RetainedBytes::default()
        }
    );
    actual
}
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(observed(match input {
        Scenario::Inputs => inputs(),
        Scenario::WorkerNotes => worker_notes(),
        Scenario::ReservedEmpty => reserved_empty(),
        Scenario::Payloads => payloads(),
        Scenario::Records => records(),
        Scenario::Stored => stored(),
        Scenario::Staged => staged(),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
