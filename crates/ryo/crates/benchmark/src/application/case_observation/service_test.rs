//! One side's verdict is appended to the receipt with the identity of the sources it read, or nothing is written.
use super::service::write;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::{case, report};
use crate::domain::discrepancy::types::Contestant;
use crate::outbound::receipts::fixtures::fake::ReceiptsFake;
use crate::outbound::receipts::types::CaseReceipts;
use crate::outbound::suite::fixtures::fake::SuiteFake;
use ryo_benchmark_contracts::domain::documents::types::text::value::{ContestantSide, YesNo};
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;

/// What the receipt sink and the case sources are.
#[derive(Clone, Copy)]
enum Scenario {
    /// No receipt file is chosen.
    NoSink,
    /// A receipt file is chosen and the case sources exist.
    Recorded,
    /// A receipt file is chosen and the case sources are missing.
    SourcesMissing,
}

/// What the receipt holds afterwards.
#[derive(Debug, PartialEq, Eq)]
enum Receipt {
    /// No row and no file.
    Untouched,
    /// One row for this side with this verdict.
    Row(ContestantSide, YesNo),
}

/// What a refused write states.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    SourceIdentityUnavailable,
    Unexpected,
}

type Input = Scenario;
type Value = Receipt;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("without a receipt sink nothing is written"),
        word: Word::Succeeds,
        input: || Scenario::NoSink,
        expect: Expect::Value(Receipt::Untouched),
    },
    Case {
        name: CaseName("a passing reference side is appended as one passing row"),
        word: Word::Succeeds,
        input: || Scenario::Recorded,
        expect: Expect::Value(Receipt::Row(ContestantSide::Reference, YesNo::Yes)),
    },
    Case {
        name: CaseName("missing case sources keep the row, mark it failed and refuse the write"),
        word: Word::Refuses,
        input: || Scenario::SourcesMissing,
        expect: Expect::Error(Refusal::SourceIdentityUnavailable),
    },
];

fn prepared(root: &std::path::Path, sources: bool) -> PreparedCase {
    if sources {
        std::fs::write(root.join("case.py"), "x = 1\n").unwrap();
    }
    let mut prepared = case();
    prepared.request.root = root.to_path_buf();
    prepared
}

fn rows(receipt: &std::path::Path) -> Vec<(ContestantSide, YesNo)> {
    use crate::outbound::inventory::interface::InventoryPort as _;
    use crate::outbound::receipts::interface::ReceiptsPort as _;
    crate::outbound::receipts::factory::local(CaseReceipts::Unrecorded)
        .case_observations(crate::outbound::receipts::types::ReceiptPath(receipt))
        .ok()
        .filter(|records| {
            crate::outbound::inventory::factory::local()
                .raw_artifacts(receipt, records)
                .is_ok()
        })
        .map(|records| {
            records
                .iter()
                .map(|row| (row.side, row.case_passed))
                .collect()
        })
        .unwrap_or_default()
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().unwrap();
    let receipt = root.path().join("cases.jsonl");
    let prepared = prepared(root.path(), !matches!(scenario, Scenario::SourcesMissing));
    let suite = SuiteFake::default();
    let receipts = ReceiptsFake {
        case_receipts: if matches!(scenario, Scenario::NoSink) {
            CaseReceipts::Unrecorded
        } else {
            CaseReceipts::Recorded(receipt.clone())
        },
    };
    let answer = write(
        &suite,
        &receipts,
        &crate::outbound::context::factory::local(),
        &prepared,
        Contestant::Reference,
        Some(&report()),
        None,
        YesNo::Yes,
        None,
        &crate::domain::process_failure::types::CaptureRecord::Absent,
    );
    match scenario {
        Scenario::NoSink => {
            answer.expect("no sink is not an error");
            assert!(!receipt.exists(), "no sink must write no file");
            Ok(Receipt::Untouched)
        }
        Scenario::Recorded => {
            answer.expect("recorded");
            let [(side, passed)] = rows(&receipt)[..] else {
                panic!("exactly one row is appended")
            };
            Ok(Receipt::Row(side, passed))
        }
        Scenario::SourcesMissing => {
            assert_eq!(
                rows(&receipt),
                vec![(ContestantSide::Reference, YesNo::No)],
                "the failed row is retained"
            );
            answer.map(|()| Receipt::Untouched).map_err(|error| {
                if error.to_string().contains("source identity unavailable") {
                    Refusal::SourceIdentityUnavailable
                } else {
                    Refusal::Unexpected
                }
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
