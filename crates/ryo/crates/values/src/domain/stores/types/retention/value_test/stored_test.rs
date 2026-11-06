//! Retained bytes of the record, stored and staged owners: exact component capacities.
use crate::domain::stores::types::retention::value::RetainedBytes;
use super::{bytes, notes, observed, proof};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    files::types::{FileNumber, FileSize, Nanoseconds},
    records::types::{
        FileOutcome, FindingRows, GoverningRules, Handed, HandedNotes, HandedWords, Handing, Ran,
        Record, RecordWords, RuleFrontier, StagedWords, StagingSession,
    },
    stored_facts::types::{ArenaBytes, Bucket, Digest, PackedWords, StoredPartial},
    stores::types::{
        HandedRows, KeptRows, StagedRow, StoredCovered, StoredFileNumber, StoredHanded,
        StoredNotes, StoredText, StoredWords,
    },
};
use ryo_language::domain::files::types::FilePath;
#[derive(Clone, Copy)]
enum Scenario {
    Records,
    Stored,
    Staged,
}
type Input = Scenario;
type Value = [u64; 4];
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
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
            offset: crate::domain::records::types::PayloadOffset(0),
            length: crate::domain::records::types::PayloadLength(11),
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
            rows: crate::domain::rows::types::RowCount(FindingRows::default()),
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
            objects: crate::domain::stores::types::ObjectCount(1),
            metadata: crate::domain::stores::types::CapacityBytes(
                owner.capacity() * size_of::<Record>() + nested_metadata
            ),
            packed_live: crate::domain::stores::types::LiveBytes(5),
            packed_capacity,
            staged_objects: crate::domain::stores::types::ObjectCount(1),
            staged_payload_bytes: crate::domain::records::types::PayloadLength(11),
            text_live: crate::domain::stores::types::LiveBytes(3),
            text_capacity,
            notes_live: crate::domain::stores::types::LiveBytes(4),
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
    let mut owner = StoredHanded(HandedRows {
        frontier: Vec::with_capacity(3),
        texts: Vec::with_capacity(4),
        notes: Vec::with_capacity(5),
        covered: Vec::with_capacity(6),
        ..HandedRows::default()
    });
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
            objects: crate::domain::stores::types::ObjectCount(1),
            metadata: crate::domain::stores::types::CapacityBytes(
                owner.frontier.capacity() * size_of::<StoredWords>()
                    + owner.texts.capacity() * size_of::<StoredText>()
                    + owner.notes.capacity() * size_of::<StoredNotes>()
                    + owner.covered.capacity() * size_of::<StoredCovered>()
                    + owner.covered[0].governing.0.capacity() * size_of::<u32>()
                    + owner.covered[0].partials.capacity() * size_of::<StoredPartial>()
            ),
            packed_live: crate::domain::stores::types::LiveBytes(5),
            packed_capacity: crate::domain::stores::types::CapacityBytes(
                owner.frontier[0].words.0.capacity()
            ),
            text_live: crate::domain::stores::types::LiveBytes(3),
            text_capacity: crate::domain::stores::types::CapacityBytes(
                owner.texts[0].text.0.capacity()
            ),
            notes_live: crate::domain::stores::types::LiveBytes(4),
            notes_capacity: crate::domain::stores::types::CapacityBytes(
                owner.notes[0].notes.notes.0.capacity()
            ),
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
            ordinal: crate::domain::records::types::StagedOrdinal(ordinal),
            words: StagedWords {
                session: StagingSession(1),
                offset: crate::domain::records::types::PayloadOffset(0),
                length: crate::domain::records::types::PayloadLength(length),
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
            objects: crate::domain::stores::types::ObjectCount(2),
            metadata: crate::domain::stores::types::CapacityBytes(
                owner.staged.capacity() * size_of::<StagedRow>()
            ),
            staged_objects: crate::domain::stores::types::ObjectCount(2),
            staged_payload_bytes: crate::domain::records::types::PayloadLength(u64::MAX),
            ..RetainedBytes::default()
        }
    );
    actual
}
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(observed(match input {
        Scenario::Records => records(),
        Scenario::Stored => stored(),
        Scenario::Staged => staged(),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
