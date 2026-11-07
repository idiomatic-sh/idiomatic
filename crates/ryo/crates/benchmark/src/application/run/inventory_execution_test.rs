//! Execution evidence is compared only under a declared capability and the contract asked for.
use super::compare;
use crate::application::inventory_receipts::fixtures::observed_row;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::types::{
    finding::SkippedPair,
    observation_document::ObservationDocument,
    text::{PathText, RuleName, ContestantSide},
};
use ryo_benchmark_contracts::domain::reports::types::{
    Comparison, ComparisonContract, ExecutionMetadata, RuleCoverage,
};

type Input = (ObservationDocument, ObservationDocument, ComparisonContract);
type Value = Comparison;
/// Why the evidence was refused.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    ContractUnsatisfied,
    CapabilityMissing,
    CapabilityUndeclared,
    AccountingAsserted,
    Other,
}

fn skipped() -> Vec<SkippedPair> {
    vec![SkippedPair {
        rule: RuleName("R1".into()),
        path: PathText("input".into()),
    }]
}

fn coverage() -> RuleCoverage {
    RuleCoverage {
        selected: 1,
        evaluated: 0,
        skipped: 1,
        unknown_or_unread: 0,
    }
}

fn candidate() -> ObservationDocument {
    ObservationDocument {
        files: vec![PathText("input".into())],
        skipped: Some(skipped()),
        coverage: Some(coverage()),
        ..observed_row("R1", "syntax", ContestantSide::Candidate)
    }
}

fn unavailable_reference() -> ObservationDocument {
    ObservationDocument {
        side: ContestantSide::Reference,
        skipped: Some(Vec::new()),
        coverage: None,
        execution_metadata: Some(ExecutionMetadata::Unavailable {
            reason: "findings-only CLI".into(),
        }),
        ..candidate()
    }
}

fn available_reference() -> ObservationDocument {
    candidate()
}

fn with_metadata(metadata: Option<ExecutionMetadata>) -> ObservationDocument {
    ObservationDocument {
        execution_metadata: metadata,
        ..unavailable_reference()
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an unavailable reference is incomparable under the diagnostics contract"),
        word: Word::Succeeds,
        input: || {
            (
                unavailable_reference(),
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Value(Comparison::Incomparable),
    },
    Case {
        name: CaseName(
            "matching accounting and skips are exact under the complete-report contract",
        ),
        word: Word::Succeeds,
        input: || {
            (
                available_reference(),
                candidate(),
                ComparisonContract::CompleteReport,
            )
        },
        expect: Expect::Value(Comparison::Exact),
    },
    Case {
        name: CaseName("an unavailable reference cannot satisfy the complete-report contract"),
        word: Word::Refuses,
        input: || {
            (
                unavailable_reference(),
                candidate(),
                ComparisonContract::CompleteReport,
            )
        },
        expect: Expect::Error(Failure::ContractUnsatisfied),
    },
    Case {
        name: CaseName("a reference whose coverage disagrees with the candidate is refused"),
        word: Word::Refuses,
        input: || {
            (
                ObservationDocument {
                    coverage: Some(RuleCoverage {
                        evaluated: 1,
                        skipped: 0,
                        ..coverage()
                    }),
                    ..available_reference()
                },
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(Failure::ContractUnsatisfied),
    },
    Case {
        name: CaseName("an absent capability is refused"),
        word: Word::Refuses,
        input: || {
            (
                with_metadata(None),
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(Failure::CapabilityMissing),
    },
    Case {
        name: CaseName("an unknown capability is refused"),
        word: Word::Refuses,
        input: || {
            (
                with_metadata(Some(ExecutionMetadata::Unknown)),
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(Failure::CapabilityUndeclared),
    },
    Case {
        name: CaseName("an unavailable capability without a reason is refused"),
        word: Word::Refuses,
        input: || {
            (
                with_metadata(Some(ExecutionMetadata::Unavailable {
                    reason: String::new(),
                })),
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(Failure::CapabilityUndeclared),
    },
    Case {
        name: CaseName("an unavailable capability that asserts skips is refused"),
        word: Word::Refuses,
        input: || {
            (
                ObservationDocument {
                    skipped: Some(skipped()),
                    ..unavailable_reference()
                },
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(Failure::AccountingAsserted),
    },
    Case {
        name: CaseName("a candidate with unread pairs is refused"),
        word: Word::Refuses,
        input: || {
            (
                unavailable_reference(),
                ObservationDocument {
                    coverage: Some(RuleCoverage {
                        unknown_or_unread: 1,
                        ..coverage()
                    }),
                    ..candidate()
                },
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(Failure::ContractUnsatisfied),
    },
];

fn subject((reference, candidate, contract): Input) -> Result<Value, Failure> {
    compare(&reference, &candidate, contract).map_err(|text| match text.as_str() {
        "execution evidence does not satisfy the declared contract" => Failure::ContractUnsatisfied,
        "missing or malformed execution capability" => Failure::CapabilityMissing,
        "execution capability is not explicitly declared" => Failure::CapabilityUndeclared,
        "unavailable execution capability contains asserted accounting" => {
            Failure::AccountingAsserted
        }
        _ => Failure::Other,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
