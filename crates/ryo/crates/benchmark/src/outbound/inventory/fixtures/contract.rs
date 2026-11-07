//! What every `inventory` adapter must pass.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::interface::InventoryPort;
use super::data::{row, summary};
use crate::outbound::inventory::types::ReceiptFile;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::receipts::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// The independently specified observations an inventory adapter answers.
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    /// A fresh receipt directory captures its streams and reports no receipt.
    CaptureWithoutReceipt,
    /// A terminal row is kept beside its observations.
    ResultIsKept,
    /// A raw artifact read over no rows is empty.
    RawArtifactsOfNoRows,
    /// The audit answer is one line.
    AuditIsOneLine,
    /// An output that already holds a summary is refused.
    PublishedOutputIsRefused,
    /// A directory that already holds receipts is refused.
    CapturedDirectoryIsRefused,
    /// A missing registration is refused.
    MissingRegistrationIsRefused,
    /// A malformed invocation record is refused.
    MalformedInvocationIsRefused,
    /// A missing context is refused.
    MissingContextIsRefused,
}

/// What a contract row is refused with.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The adapter refused the input as invalid.
    Invalid,
    /// The adapter failed for another reason.
    Other,
}

/// The operation a row exercises.
type Input = Operation;
/// What a row expects when it succeeds.
type Value = ();
/// What a row expects when it is refused.
type Failure = Refusal;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a captured directory reports no receipt"),
        word: Word::Succeeds,
        input: || Operation::CaptureWithoutReceipt,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a terminal row is kept"),
        word: Word::Succeeds,
        input: || Operation::ResultIsKept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("no rows reference no raw artifacts"),
        word: Word::Succeeds,
        input: || Operation::RawArtifactsOfNoRows,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("the audit answer is one line"),
        word: Word::Succeeds,
        input: || Operation::AuditIsOneLine,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a published output is refused a second beginning"),
        word: Word::Refuses,
        input: || Operation::PublishedOutputIsRefused,
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a captured directory is refused reuse"),
        word: Word::Refuses,
        input: || Operation::CapturedDirectoryIsRefused,
        expect: Expect::Error(Refusal::Invalid),
    },
    Case {
        name: CaseName("a missing registration is refused"),
        word: Word::Refuses,
        input: || Operation::MissingRegistrationIsRefused,
        expect: Expect::Error(Refusal::Other),
    },
    Case {
        name: CaseName("a malformed invocation record is refused"),
        word: Word::Refuses,
        input: || Operation::MalformedInvocationIsRefused,
        expect: Expect::Error(Refusal::Other),
    },
    Case {
        name: CaseName("a missing context is refused"),
        word: Word::Refuses,
        input: || Operation::MissingContextIsRefused,
        expect: Expect::Error(Refusal::Other),
    },
];

/// Run the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
/// Panics if any contract observation differs.
pub fn check(adapter: &impl InventoryPort) {
    let subject = |operation| {
        observe(adapter, operation).map_err(|error| match error {
            Error::Invalid(_) => Refusal::Invalid,
            Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Other,
        })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

/// Exercise one operation against independently constructed inputs.
fn observe(adapter: &impl InventoryPort, operation: Operation) -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let observations = scratch.path().join("observations");
    let streams = CapturedBytes(b"stream".to_vec());
    match operation {
        Operation::CaptureWithoutReceipt => {
            adapter.prepare(&observations)?;
            match adapter.capture(&observations, &streams, &streams)? {
                ReceiptFile::Absent => Ok(()),
                ReceiptFile::Present(_) => {
                    Err(Error::Invalid("a receipt appeared from nothing".into()))
                }
            }
        }
        Operation::ResultIsKept => adapter.result(&observations, &row()),
        Operation::RawArtifactsOfNoRows => adapter
            .raw_artifacts(&scratch.path().join("cases.jsonl"), &[])
            .map(|_| ()),
        Operation::AuditIsOneLine => {
            let answer = adapter.audit_report(&ReceiptAudit::default())?;
            if answer.0.last() == Some(&b'\n') {
                Ok(())
            } else {
                Err(Error::Invalid("the audit answer is not one line".into()))
            }
        }
        Operation::PublishedOutputIsRefused => {
            adapter.publish(scratch.path(), &summary())?;
            adapter.begin(scratch.path())
        }
        Operation::CapturedDirectoryIsRefused => {
            adapter.prepare(&observations)?;
            let _receipt = adapter.capture(&observations, &streams, &streams)?;
            adapter.prepare(&observations)
        }
        Operation::MissingRegistrationIsRefused => adapter
            .load(&scratch.path().join("registration.json"))
            .map(|_| ())
            .map_err(|error| Error::Io(std::io::Error::other(error.to_string()))),
        Operation::MalformedInvocationIsRefused => adapter
            .decode_invocation(&streams)
            .map(|_| ())
            .map_err(|error| Error::Io(std::io::Error::other(error.to_string()))),
        Operation::MissingContextIsRefused => adapter
            .context_files(&scratch.path().join("context"))
            .map(|_| ())
            .map_err(|error| Error::Io(std::io::Error::other(error.to_string()))),
    }
}
