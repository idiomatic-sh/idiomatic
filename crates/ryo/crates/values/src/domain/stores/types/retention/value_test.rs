//! Owning-container capacities, live payloads and descriptor-only storage have independent exact expectations.
use super::value::RetainedBytes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    records::types::{FrontierCoverage, FrontierProof},
    rows::types::{RowCount, Table},
    stored_facts::types::{
        ArenaBytes, Digest, FileNotes, KeptWords, LanguageNumber, NoteBytes, PackedWords,
    },
};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
#[derive(Clone, Copy)]
enum Scenario {
    Inputs,
    WorkerNotes,
    ReservedEmpty,
    Payloads,
}
type Input = Scenario;
/// The retained objects, rows, live payload bytes and staged payload bytes.
pub(in crate::domain::stores) type Value = (usize, usize, usize, u64);
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("input_metadata_comes_from_actual_owner_capacity_and_embedded_tables"),
        word: Word::Succeeds,
        input: || Scenario::Inputs,
        expect: Expect::Value((2, 3, 4 * size_of::<Cell>(), 0)),
    },
    Case {
        name: CaseName("worker_note_metadata_uses_owner_layout_and_preserves_note_capacities"),
        word: Word::Succeeds,
        input: || Scenario::WorkerNotes,
        expect: Expect::Value((2, 0, 7, 0)),
    },
    Case {
        name: CaseName("empty_owners_still_report_reserved_metadata_allocations"),
        word: Word::Succeeds,
        input: || Scenario::ReservedEmpty,
        expect: Expect::Value((0, 0, 0, 0)),
    },
    Case {
        name: CaseName("packed_arena_and_table_payloads_keep_live_and_capacity_separate"),
        word: Word::Succeeds,
        input: || Scenario::Payloads,
        expect: Expect::Value((1, 2, 4 * size_of::<Cell>(), 0)),
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
pub(in crate::domain::stores) fn bytes(capacity: usize, live: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(capacity);
    bytes.resize(live, 1);
    bytes
}
pub(in crate::domain::stores) fn table(
    kind_capacity: usize,
    cell_capacity: usize,
    width: usize,
    rows: usize,
) -> Table {
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
pub(in crate::domain::stores) fn notes(capacity: usize, live: usize) -> FileNotes {
    FileNotes {
        language: LanguageNumber(0),
        notes: NoteBytes(bytes(capacity, live)),
    }
}
pub(in crate::domain::stores) fn proof() -> FrontierProof {
    FrontierProof {
        rule: crate::domain::records::types::ProducerRule(0),
        input: crate::domain::records::types::FrontierOrdinal(0),
        rule_digest: Digest([0; 16]),
        coverage: FrontierCoverage::Complete,
    }
}
pub(in crate::domain::stores) fn observed(bytes: RetainedBytes) -> Value {
    (
        bytes.objects.0,
        bytes.rows.0,
        bytes.cells_live.0 + bytes.packed_live.0 + bytes.text_live.0 + bytes.notes_live.0,
        bytes.staged_payload_bytes.0,
    )
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
            objects: crate::domain::stores::types::ObjectCount(2),
            metadata: crate::domain::stores::types::CapacityBytes(
                owner.capacity() * size_of::<InputOwner>()
                    + (owner[0].table.kinds.capacity() + owner[1].table.kinds.capacity())
                        * size_of::<CellKind>()
            ),
            rows: crate::domain::rows::types::RowCount(3),
            cells_live: crate::domain::stores::types::LiveBytes(4 * size_of::<Cell>()),
            cells_capacity: crate::domain::stores::types::CapacityBytes(
                (owner[0].table.cells.capacity() + owner[1].table.cells.capacity())
                    * size_of::<Cell>()
            ),
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
            objects: crate::domain::stores::types::ObjectCount(2),
            metadata: crate::domain::stores::types::CapacityBytes(
                owner.capacity() * size_of::<NoteOwner>()
            ),
            notes_live: crate::domain::stores::types::LiveBytes(7),
            notes_capacity: crate::domain::stores::types::CapacityBytes(
                owner[0].notes.notes.0.capacity() + owner[1].notes.notes.0.capacity()
            ),
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
            metadata: crate::domain::stores::types::CapacityBytes(
                inputs.capacity() * size_of::<InputOwner>()
            ),
            ..RetainedBytes::default()
        }
    );
    assert_eq!(
        noted,
        RetainedBytes {
            metadata: crate::domain::stores::types::CapacityBytes(
                notes.capacity() * size_of::<NoteOwner>()
            ),
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
            objects: crate::domain::stores::types::ObjectCount(2),
            metadata: crate::domain::stores::types::CapacityBytes(
                arenas.capacity() * size_of::<ArenaBytes>()
            ),
            text_live: crate::domain::stores::types::LiveBytes(7),
            text_capacity: crate::domain::stores::types::CapacityBytes(
                arenas[0].0.capacity() + arenas[1].0.capacity()
            ),
            ..RetainedBytes::default()
        }
    );
    let packed = KeptWords(PackedWords(bytes(23, 5)));
    let pointer = packed.0.0.as_ptr();
    assert_eq!(
        RetainedBytes::packed(&packed),
        RetainedBytes {
            objects: crate::domain::stores::types::ObjectCount(1),
            packed_live: crate::domain::stores::types::LiveBytes(5),
            packed_capacity: crate::domain::stores::types::CapacityBytes(packed.0.0.capacity()),
            ..RetainedBytes::default()
        }
    );
    assert_eq!(packed.0.0.as_ptr(), pointer);
    let table = table(5, 11, 2, 2);
    let actual = RetainedBytes::table(&table);
    assert_eq!(
        actual,
        RetainedBytes {
            objects: crate::domain::stores::types::ObjectCount(1),
            metadata: crate::domain::stores::types::CapacityBytes(
                table.kinds.capacity() * size_of::<CellKind>()
            ),
            rows: crate::domain::rows::types::RowCount(2),
            cells_live: crate::domain::stores::types::LiveBytes(4 * size_of::<Cell>()),
            cells_capacity: crate::domain::stores::types::CapacityBytes(
                table.cells.capacity() * size_of::<Cell>()
            ),
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
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
