//! Behavioral proof of `inventory_receipts` over registered cases and native rows.
use ryo_benchmark_contracts::domain::{
    documents::types::{
        observation_document::{
            ExitStatus, ProcessObservation, ReferenceExpectation, RuleContextRecord,
        },
        reference_discrepancy::{
            DiscrepancyIdentity, ExpectationKind, ObservedExecutable, ReferenceDiscrepancy,
        },
        text::{
            CanonicalJson, CaseName as DocumentCaseName, PathText, ReasonText, RuleName,
            SchemaName, Sha256Text, YesNo,
        },
    },
    reports::types::{ExecutionMetadata, RuleCoverage},
    workloads::types::DiscrepancyBasis,
};

use ryo_benchmark_contracts::domain::{
    measurements::types::ProcessExitCode, process::types::ProcessFailure,
};

use super::{
    errors::InventoryReceiptsError,
    fixtures::{self, Documents},
    service::validate,
    types::{ReceiptCounts, Tally},
};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::inventory_comparison::errors::InventoryComparisonError;

type Input = Documents;
type Value = ReceiptCounts;
type Failure = InventoryReceiptsError;

const COHORT: &[&str] = &["A001", "B002"];
const DECLARED: &str = "{\"A001\":[{\"name\":\"matched\"}]}";

const fn counts(raw_exact: usize, documented: usize, contract_only: usize) -> ReceiptCounts {
    ReceiptCounts {
        raw_exact: Tally(raw_exact),
        documented: Tally(documented),
        contract_only: Tally(contract_only),
    }
}

/// A discrepancy registered as an explicit process failure.
fn process_discrepancy() -> ReferenceDiscrepancy {
    ReferenceDiscrepancy {
        basis: DiscrepancyBasis::LanguageDefect,
        identity: DiscrepancyIdentity {
            tool: ReasonText("tool".into()),
            version: ReasonText("1".into()),
            observed_executable_sha256: ObservedExecutable::NotObserved,
            profile: Sha256Text("profile-pin".into()),
            settings: Sha256Text("settings-pin".into()),
            source: Sha256Text("source-pin".into()),
            reference_output: Sha256Text("reference-pin".into()),
            intended_output: Sha256Text("intended-pin".into()),
            file_metadata: None,
        },
        approved_executable_sha256: Vec::new(),
        reference_expected: ReferenceExpectation::Underivable,
        intended_expected: Vec::new(),
        reason: ReasonText("process failure".into()),
        process: Some(ProcessFailure {
            exit: ProcessExitCode(1),
            stdout: Vec::new(),
            stderr: Vec::new(),
            observations: Vec::new(),
        }),
    }
}

/// A reference process observed instead of diagnostics.
fn process_observation() -> ProcessObservation {
    ProcessObservation {
        kind: ExpectationKind::ProcessFailure,
        exit: ExitStatus(1),
        signal: None,
        actual_request_root: PathText("/cases/root".into()),
        observer_no_deadline: YesNo::Yes,
        executable_sha256: None,
        settings_sha256: None,
        metadata_error: None,
        template_sha256: None,
        artifacts: None,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two complete passing sides are raw exact"),
        word: Word::Succeeds,
        input: fixtures::passing,
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("a declared non-executable file mode is bound to both sides"),
        word: Word::Succeeds,
        input: || fixtures::with_file_metadata(false),
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("a declared executable file mode is bound to both sides"),
        word: Word::Succeeds,
        input: || fixtures::with_file_metadata(true),
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("an unexpected candidate diagnostic is refused"),
        word: Word::Refuses,
        input: fixtures::candidate_with_unexpected_diagnostic,
        expect: Expect::Error(InventoryReceiptsError::DiagnosticsDiffer),
    },
    Case {
        name: CaseName("a candidate without diagnostics is refused"),
        word: Word::Refuses,
        input: fixtures::candidate_without_diagnostics,
        expect: Expect::Error(InventoryReceiptsError::DiagnosticsMissing),
    },
    Case {
        name: CaseName("a case registered twice is refused"),
        word: Word::Refuses,
        input: fixtures::duplicate_registration,
        expect: Expect::Error(InventoryReceiptsError::DuplicateRegisteredCase),
    },
    Case {
        name: CaseName("rows for a case that was never registered are refused"),
        word: Word::Refuses,
        input: fixtures::unregistered_observation,
        expect: Expect::Error(InventoryReceiptsError::UnregisteredObservation),
    },
    Case {
        name: CaseName("rows observed twice are refused"),
        word: Word::Refuses,
        input: fixtures::duplicate_observation,
        expect: Expect::Error(InventoryReceiptsError::DuplicateObservation),
    },
    Case {
        name: CaseName("rows observed for the other file mode are refused"),
        word: Word::Refuses,
        input: fixtures::declaring_the_other_mode,
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a missing registered rule identity is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.0[0].rule = RuleName(String::new())),
        expect: Expect::Error(InventoryReceiptsError::RegisteredRuleMissing),
    },
    Case {
        name: CaseName("a missing registered case identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].name = DocumentCaseName(String::new())
            })
        },
        expect: Expect::Error(InventoryReceiptsError::RegisteredCaseMissing),
    },
    Case {
        name: CaseName("a missing native row is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                let _dropped = d.1.pop();
            })
        },
        expect: Expect::Error(InventoryReceiptsError::NativeCaseReceiptCount),
    },
    Case {
        name: CaseName("an expected diagnostic that ends before it starts is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].expected = vec![fixtures::finding("A001", "input.txt", 2, 1)]
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiagnosticSpanOrIdentityInvalid),
    },
    Case {
        name: CaseName("a reference whose execution capability is unknown cannot be compared"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].execution_metadata = Some(ExecutionMetadata::Unknown)
            })
        },
        expect: Expect::Error(InventoryReceiptsError::Comparison(
            InventoryComparisonError::UndeclaredExecutionCapability,
        )),
    },
    Case {
        name: CaseName("an intended expectation that differs from the registered one is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].reference_discrepancy = Some(ReferenceDiscrepancy {
                    intended_expected: vec![fixtures::finding("A001", "input.txt", 0, 1)],
                    process: None,
                    ..process_discrepancy()
                })
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyIntentionDiffers),
    },
    Case {
        name: CaseName("a rule context that differs from the registration is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].rule_context =
                    RuleContextRecord::Declared(CanonicalJson("{\"configured\":true}".into()))
            })
        },
        expect: Expect::Error(InventoryReceiptsError::RuleContextDiffers),
    },
    Case {
        name: CaseName("an observation schema that differs from the registration is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].observation_schema = SchemaName("other".into())
            })
        },
        expect: Expect::Error(InventoryReceiptsError::ExpectationContractDiffers),
    },
    Case {
        name: CaseName("a side that did not complete is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.1[1].complete = YesNo::No),
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a candidate that does not account for every pair is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[1].coverage = Some(RuleCoverage {
                    selected: ryo_benchmark_contracts::domain::reports::types::PairCount(2),
                    evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                    skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                    unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(
                        0,
                    ),
                })
            })
        },
        expect: Expect::Error(InventoryReceiptsError::CandidateNotAccounted),
    },
];

fn subject((cases, rows): Input) -> Result<Value, Failure> {
    validate(&cases, &rows)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod discrepancy_test;

#[cfg(test)]
mod identity_test;
