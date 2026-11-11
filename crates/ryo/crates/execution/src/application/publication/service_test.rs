//! Complete and refused producer publication through the application service.
use super::{
    errors::PublicationError,
    fixtures::{Produced, Published, Publishing, Refused, Worker, publish},
    service::publication,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::finish::types::PublicationReason;
use ryo_relations::domain::relations::types::{FileId, NamePlace, Offset, Origin, WorkerNumber};
use std::borrow::Cow;

type Input = Publishing;
type Value = Published;
type Failure = Refused;

/// What a publication that leaves the cell as it was leaves behind: the arena as it was, no place.
const UNTOUCHED: Published = Published {
    named: false,
    unchanged: true,
    arena: Cow::Borrowed(&[7]),
    places: Cow::Borrowed(&[]),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(numeric_row_is_unchanged)),
        word: Word::Succeeds,
        input: || Publishing {
            cell: Produced::Number,
            worker: Worker::Producing,
        },
        expect: Expect::Value(UNTOUCHED),
    },
    Case {
        name: CaseName(stringify!(symbol_becomes_a_portable_name)),
        word: Word::Succeeds,
        input: || Publishing {
            cell: Produced::Symbol,
            worker: Worker::Producing,
        },
        // The arena gains the name's bytes after its `7`, and one place points at them in worker 0's text.
        expect: Expect::Value(Published {
            named: true,
            unchanged: false,
            arena: Cow::Borrowed(&[7, b'a', b'l', b'p', b'h', b'a']),
            places: Cow::Borrowed(&[NamePlace {
                origin: Origin::Derived(WorkerNumber(0)),
                // A derived name is read from no file.
                file: FileId(u32::MAX),
                start: Offset(1),
                end: Offset(6),
            }]),
        }),
    },
    Case {
        name: CaseName(stringify!(reserved_worker_refuses_without_mutation)),
        word: Word::Refuses,
        input: || Publishing {
            cell: Produced::Number,
            worker: Worker::Reserved,
        },
        expect: Expect::Error(Refused {
            error: PublicationError::Publication {
                reason: PublicationReason::ReservedArena,
            },
            message: Cow::Borrowed(
                "cross publication requires an ordinary arena slot and fresh place log",
            ),
            left: UNTOUCHED,
        }),
    },
];

/// Publishes through the application service.
fn subject(publishing: Input) -> Result<Value, Failure> {
    publish(publishing, publication)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
