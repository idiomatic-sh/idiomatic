//! Execution evidence is compared only under a declared capability and the contract asked for.
use super::errors::InventoryComparisonError;
use super::service::compare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::inventory_receipts::fixtures::observed_row;
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::documents::types::finding::value::SkippedPair;
use ryo_benchmark_contracts::domain::documents::types::text::value::{ContestantSide, ExitStatus, PathText, Pin, RuleName, Signal, YesNo};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    ObservationDocument, ProcessArtifactsRecord, ProcessRecord, RecordedMetadata, RecordedReason, ReferenceProcess,
    ReportedSkips,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::ExpectationKind;
use ryo_benchmark_contracts::domain::reports::types::{
    Comparison, ComparisonContract, CoverageAccounting, ExecutionMetadata, RuleCoverage,
};

type Input = (ObservationDocument, ObservationDocument, ComparisonContract);
type Value = Comparison;
type Failure = InventoryComparisonError;

fn skipped() -> Vec<SkippedPair> {
    vec![SkippedPair {
        rule: RuleName("R1".into()),
        path: PathText("input".into()),
    }]
}

fn coverage() -> RuleCoverage {
    RuleCoverage {
        selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
        evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
        skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
        unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
    }
}

fn candidate() -> ObservationDocument {
    ObservationDocument {
        files: vec![PathText("input".into())],
        skipped: ReportedSkips::Reported(skipped()),
        coverage: CoverageAccounting::Accounted(coverage()),
        ..observed_row("R1", "syntax", ContestantSide::Candidate)
    }
}

fn unavailable_reference() -> ObservationDocument {
    ObservationDocument {
        side: ContestantSide::Reference,
        skipped: ReportedSkips::Reported(Vec::new()),
        coverage: CoverageAccounting::Unaccounted,
        execution_metadata: RecordedMetadata::Recorded(ExecutionMetadata::Unavailable {
            reason: ryo_benchmark_contracts::domain::reports::types::UnavailableReason("findings-only CLI".into()),
        }),
        ..candidate()
    }
}

fn available_reference() -> ObservationDocument {
    candidate()
}

/// A reference process observed instead of diagnostics.
fn process_observation() -> ProcessRecord {
    ProcessRecord {
        kind: ExpectationKind::ProcessFailure,
        exit: ExitStatus(1),
        signal: Signal::Absent,
        actual_request_root: PathText("/cases/root".into()),
        observer_no_deadline: YesNo::Yes,
        executable_sha256: Pin::Absent,
        settings_sha256: Pin::Absent,
        metadata_error: RecordedReason::Absent,
        template_sha256: Pin::Absent,
        artifacts: ProcessArtifactsRecord::Unattached,
    }
}

fn process_reference() -> ObservationDocument {
    ObservationDocument {
        side: ContestantSide::Reference,
        process_observation: ReferenceProcess::Observed(process_observation()),
        ..candidate()
    }
}

fn with_metadata(metadata: RecordedMetadata) -> ObservationDocument {
    ObservationDocument {
        execution_metadata: metadata,
        ..unavailable_reference()
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an observed file without a path is refused"),
        word: Word::Refuses,
        input: || {
            (
                ObservationDocument {
                    files: vec![PathText(String::new())],
                    ..available_reference()
                },
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::InvalidExecutionFile),
    },
    Case {
        name: CaseName("an observation without a skip list is refused"),
        word: Word::Refuses,
        input: || {
            (
                ObservationDocument {
                    skipped: ReportedSkips::Unreported,
                    ..available_reference()
                },
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::MissingExecutionSkips),
    },
    Case {
        name: CaseName("a skipped rule without an identity is refused"),
        word: Word::Refuses,
        input: || {
            (
                ObservationDocument {
                    skipped: ReportedSkips::Reported(vec![SkippedPair {
                        rule: RuleName(String::new()),
                        path: PathText("input".into()),
                    }]),
                    ..available_reference()
                },
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::InvalidSkippedRule),
    },
    Case {
        name: CaseName("a process failure under a complete-report contract is refused"),
        word: Word::Refuses,
        input: || {
            (
                process_reference(),
                candidate(),
                ComparisonContract::CompleteReport,
            )
        },
        expect: Expect::Error(InventoryComparisonError::ProcessFailureHasNoCompleteReport),
    },
    Case {
        name: CaseName(
            "a process failure against an unaccounted candidate is refused with its own words",
        ),
        word: Word::Refuses,
        input: || {
            (
                process_reference(),
                ObservationDocument {
                    coverage: CoverageAccounting::Unaccounted,
                    ..candidate()
                },
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::CandidateExecutionIncomplete(
            RefusalText(String::new()),
        )),
    },
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
        expect: Expect::Error(InventoryComparisonError::ExecutionEvidenceUnsatisfied),
    },
    Case {
        name: CaseName("a reference whose coverage disagrees with the candidate is refused"),
        word: Word::Refuses,
        input: || {
            (
                ObservationDocument {
                    coverage: CoverageAccounting::Accounted(RuleCoverage {
                        evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                        skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                        ..coverage()
                    }),
                    ..available_reference()
                },
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::ExecutionEvidenceUnsatisfied),
    },
    Case {
        name: CaseName("an absent capability is refused"),
        word: Word::Refuses,
        input: || {
            (
                with_metadata(RecordedMetadata::Unrecorded),
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::MissingExecutionCapability),
    },
    Case {
        name: CaseName("an unknown capability is refused"),
        word: Word::Refuses,
        input: || {
            (
                with_metadata(RecordedMetadata::Recorded(ExecutionMetadata::Unknown)),
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::UndeclaredExecutionCapability),
    },
    Case {
        name: CaseName("an unavailable capability without a reason is refused"),
        word: Word::Refuses,
        input: || {
            (
                with_metadata(RecordedMetadata::Recorded(ExecutionMetadata::Unavailable {
                    reason: ryo_benchmark_contracts::domain::reports::types::UnavailableReason(String::new()),
                })),
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::UndeclaredExecutionCapability),
    },
    Case {
        name: CaseName("an unavailable capability that asserts skips is refused"),
        word: Word::Refuses,
        input: || {
            (
                ObservationDocument {
                    skipped: ReportedSkips::Reported(skipped()),
                    ..unavailable_reference()
                },
                candidate(),
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::UnavailableCapabilityAsserted),
    },
    Case {
        name: CaseName("a candidate with unread pairs is refused"),
        word: Word::Refuses,
        input: || {
            (
                unavailable_reference(),
                ObservationDocument {
                    coverage: CoverageAccounting::Accounted(RuleCoverage {
                        unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                        ..coverage()
                    }),
                    ..candidate()
                },
                ComparisonContract::Diagnostics,
            )
        },
        expect: Expect::Error(InventoryComparisonError::ExecutionEvidenceUnsatisfied),
    },
];

/// A refusal in a library's words is one kind of refusal, whatever the words.
fn words_dropped(error: Failure) -> Failure {
    match error {
        InventoryComparisonError::CandidateExecutionIncomplete(_) => {
            InventoryComparisonError::CandidateExecutionIncomplete(RefusalText(String::new()))
        }
        other => other,
    }
}

fn subject((reference, candidate, contract): Input) -> Result<Value, Failure> {
    compare(&reference, &candidate, contract).map_err(words_dropped)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
