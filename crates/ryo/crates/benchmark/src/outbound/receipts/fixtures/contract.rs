//! What every `receipts` adapter must pass.
use super::samples::observation;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::receipts::interface::ReceiptsPort;
use crate::outbound::receipts::types::{CaseReceipts, ReceiptPath};

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Destination,
    RecordTwice,
    UnsafeName,
    ReadBack,
    ReadMissing,
}
type Input = Operation;
type Value = usize;
type Failure = ();

pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the destination the edge chose is the destination reported"),
        word: Word::Succeeds,
        input: || Operation::Destination,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("two recorded observations are two rows of the receipt"),
        word: Word::Succeeds,
        input: || Operation::RecordTwice,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a recorded row reads back as one observation"),
        word: Word::Succeeds,
        input: || Operation::ReadBack,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a missing receipt is refused"),
        word: Word::Refuses,
        input: || Operation::ReadMissing,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("a receipt name that is not a plain file name is refused"),
        word: Word::Refuses,
        input: || Operation::UnsafeName,
        expect: Expect::Error(()),
    },
];

/// Exercise actual receipt appends; the rows' content is proved by the application tests.
pub fn check(adapter: &impl ReceiptsPort, destination: &CaseReceipts) {
    let temporary = tempfile::tempdir().expect("receipts fixture");
    let subject = |operation| match operation {
        Operation::Destination => Ok(usize::from(&adapter.case_receipts() == destination)),
        Operation::RecordTwice => {
            let receipt = temporary.path().join("rows.jsonl");
            for _ in 0..2 {
                adapter
                    .record_case(ReceiptPath(&receipt), observation(), None)
                    .map_err(|_refused| ())?;
            }
            Ok(std::fs::read_to_string(&receipt)
                .map_err(|_unreadable| ())?
                .lines()
                .count())
        }
        Operation::ReadBack => {
            let receipt = temporary.path().join("read.jsonl");
            adapter
                .record_case(ReceiptPath(&receipt), observation(), None)
                .and_then(|()| adapter.case_observations(ReceiptPath(&receipt)))
                .map(|rows| rows.len())
                .map_err(|_refused| ())
        }
        Operation::ReadMissing => adapter
            .case_observations(ReceiptPath(&temporary.path().join("missing.jsonl")))
            .map(|rows| rows.len())
            .map_err(|_refused| ()),
        Operation::UnsafeName => adapter
            .record_case(
                ReceiptPath(&temporary.path().join("bad:name.jsonl")),
                observation(),
                None,
            )
            .map(|()| 0)
            .map_err(|_refused| ()),
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
