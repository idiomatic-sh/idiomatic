//! Behavioral proof of `inventory_receipts` over registered cases and native rows.
use super::errors::InventoryReceiptsError;
use super::fixtures::{self, Documents};
use super::service::validate;
use super::types::{ReceiptCounts, Tally};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::registered_inputs::errors::RegisteredInputsError;
use crate::outbound::inventory::types::case_document::EntryState;
use crate::outbound::suite::types::{
    observation_document::{ContextManifest, ContextMode, ExitStatus, ProcessObservation},
    process::ProcessFailureWire,
    reference_discrepancy::{DiscrepancyIdentity, ExpectationKind, ReferenceDiscrepancy},
    text::{
        CanonicalJson, CaseName as DocumentCaseName, PathText, ReasonText, RuleName, SchemaName,
        Sha256Text, YesNo,
    },
};
use ryo_benchmark_contracts::domain::reports::types::{ComparisonContract, RuleCoverage};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;

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
            observed_executable_sha256: None,
            profile: Sha256Text("profile-pin".into()),
            settings: Sha256Text("settings-pin".into()),
            source: Sha256Text("source-pin".into()),
            reference_output: Sha256Text("reference-pin".into()),
            intended_output: Sha256Text("intended-pin".into()),
            file_metadata: None,
        },
        approved_executable_sha256: Vec::new(),
        reference_expected: None,
        intended_expected: Vec::new(),
        reason: ReasonText("process failure".into()),
        process: Some(ProcessFailureWire::ProcessFailure {
            exit: 1,
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
        name: CaseName("an independently bound reference difference is a documented discrepancy"),
        word: Word::Succeeds,
        input: fixtures::independently_bound_discrepancy,
        expect: Expect::Value(counts(0, 1, 0)),
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
        name: CaseName("a case that never declared file metadata keeps its receipt contract"),
        word: Word::Succeeds,
        input: fixtures::unspecified_legacy,
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("the registered project scope binds both raw sides"),
        word: Word::Succeeds,
        input: || fixtures::project_scope(&["src"], EntryState::File, None),
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("an explicit cohort on the case and both rows is accepted"),
        word: Word::Succeeds,
        input: || fixtures::with_selected_rules(Some(COHORT), [Some(COHORT), Some(COHORT)]),
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("observation declarations carried by both rows are accepted"),
        word: Word::Succeeds,
        input: || {
            fixtures::with_rule_observations(Some(DECLARED), [Some(DECLARED), Some(DECLARED)])
        },
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
        name: CaseName("a discrepancy with an unbound settings identity is refused"),
        word: Word::Refuses,
        input: fixtures::unbound_discrepancy,
        expect: Expect::Error(InventoryReceiptsError::StaleDiscrepancyEvidence),
    },
    Case {
        name: CaseName("rows observed for the other file mode are refused"),
        word: Word::Refuses,
        input: fixtures::declaring_the_other_mode,
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a case without its metadata declaration is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(
                || fixtures::with_file_metadata(false),
                |d| d.0[0].file_metadata = None,
            )
        },
        expect: Expect::Error(InventoryReceiptsError::Inputs(
            RegisteredInputsError::InputObservationsMissing,
        )),
    },
    Case {
        name: CaseName("a case without its metadata identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(
                || fixtures::with_file_metadata(false),
                |d| d.0[0].file_metadata_sha256 = None,
            )
        },
        expect: Expect::Error(InventoryReceiptsError::FileMetadataDigestDiffers),
    },
    Case {
        name: CaseName("rows without the metadata identity are refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(
                || fixtures::with_file_metadata(false),
                |d| {
                    for row in &mut d.1 {
                        row.file_metadata_sha256 = None;
                    }
                },
            )
        },
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a changed project scope no longer matches its registered identity"),
        word: Word::Refuses,
        input: || fixtures::project_scope(&["vendor", "src"], EntryState::File, None),
        expect: Expect::Error(InventoryReceiptsError::ProjectInputsDigestDiffers),
    },
    Case {
        name: CaseName("a changed project entry no longer matches its registered identity"),
        word: Word::Refuses,
        input: || fixtures::project_scope(&["src"], EntryState::Missing, None),
        expect: Expect::Error(InventoryReceiptsError::ProjectInputsDigestDiffers),
    },
    Case {
        name: CaseName("a stale project identity on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::project_scope(&["src"], EntryState::File, Some("stale-identity")),
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a cohort narrowed on the case is refused"),
        word: Word::Refuses,
        input: || fixtures::with_selected_rules(Some(&["A001"]), [Some(COHORT), Some(COHORT)]),
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a cohort missing on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::with_selected_rules(Some(COHORT), [None, Some(COHORT)]),
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a cohort missing on the case is refused"),
        word: Word::Refuses,
        input: || fixtures::with_selected_rules(None, [Some(COHORT), Some(COHORT)]),
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a changed observation declaration on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::with_rule_observations(Some(DECLARED), [Some(DECLARED), Some("other")]),
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("an observation declaration missing on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::with_rule_observations(Some(DECLARED), [None, Some(DECLARED)]),
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("an observation declaration missing on the case is refused"),
        word: Word::Refuses,
        input: || fixtures::with_rule_observations(None, [Some(DECLARED), Some(DECLARED)]),
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
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
        name: CaseName("a process expectation under a complete-report contract is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].expectation_contract = Some(ComparisonContract::CompleteReport);
                d.0[0].reference_discrepancy = Some(process_discrepancy());
            })
        },
        expect: Expect::Error(InventoryReceiptsError::ProcessExpectationNotComplete),
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
                d.1[0].rule_context = Some(CanonicalJson("{\"configured\":true}".into()))
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
        name: CaseName("a documented discrepancy the registration never declared is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].documented_reference_discrepancy = YesNo::Yes
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyAuthorityDiffers),
    },
    Case {
        name: CaseName("a process observation inside a diagnostic report is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].process_observation = Some(process_observation())
            })
        },
        expect: Expect::Error(InventoryReceiptsError::UnexpectedProcessObservation),
    },
    Case {
        name: CaseName("a candidate that does not account for every pair is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[1].coverage = Some(RuleCoverage {
                    selected: 2,
                    evaluated: 1,
                    skipped: 0,
                    unknown_or_unread: 0,
                })
            })
        },
        expect: Expect::Error(InventoryReceiptsError::CandidateNotAccounted),
    },
    Case {
        name: CaseName("a project context receipt in an isolated run is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].context_manifest = Some(ContextManifest {
                    mode: ContextMode::ListedDisk,
                    logical_root: PathText("root".into()),
                    actual_root: PathText("/cases/root".into()),
                    manifest_hash: Sha256Text("hash".into()),
                })
            })
        },
        expect: Expect::Error(InventoryReceiptsError::UnexpectedProjectContext),
    },
    Case {
        name: CaseName("a project context receipt that no longer matches its identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].project_context_mode = ContextMode::ListedDisk;
                d.0[0].context_identity = Some(fixtures::listed_context());
            })
        },
        expect: Expect::Error(InventoryReceiptsError::StaleProjectContext),
    },
    Case {
        name: CaseName("a discrepancy bound to other file metadata is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                if let Some(certificate) = d.0[0].reference_discrepancy.as_mut() {
                    certificate.identity.file_metadata = Some(Sha256Text("other".into()));
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyMetadataDiffers),
    },
    Case {
        name: CaseName("a discrepancy validation that the sides do not share is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                d.1[1].discrepancy_validation = None
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyValidationMissing),
    },
    Case {
        name: CaseName("a discrepancy basis the sides did not verify is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                if let Some(certificate) = d.0[0].reference_discrepancy.as_mut() {
                    certificate.basis = DiscrepancyBasis::ToolContract;
                }
                for row in &mut d.1 {
                    row.documented_reference_discrepancy = YesNo::No;
                    row.documented_tool_contract_difference = Some(YesNo::Yes);
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyAuthorityUnverified),
    },
    Case {
        name: CaseName("a discrepancy without its tool identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                for row in &mut d.1 {
                    if let Some(proof) = row.discrepancy_validation.as_mut() {
                        proof.actual_executable_sha256 = Sha256Text(String::new());
                    }
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyToolMissing),
    },
    Case {
        name: CaseName("a discrepancy tool that is not independently pinned is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                if let Some(certificate) = d.0[0].reference_discrepancy.as_mut() {
                    certificate.approved_executable_sha256 = vec![Sha256Text("another-pin".into())];
                }
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyToolUnpinned),
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
