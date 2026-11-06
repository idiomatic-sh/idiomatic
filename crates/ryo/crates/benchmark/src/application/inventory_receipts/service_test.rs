//! Behavioral proof of `inventory_receipts` over registered cases and native rows.
use super::errors::InventoryReceiptsError;
use super::fixtures::{self, Documents};
use super::service::validate;
use super::types::{ReceiptCounts, Tally};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::registered_inputs::errors::RegisteredInputsError;
use serde_json::{Value as Json, json};

type Input = Documents;
type Value = ReceiptCounts;
type Failure = InventoryReceiptsError;

const fn counts(raw_exact: usize, documented: usize, contract_only: usize) -> ReceiptCounts {
    ReceiptCounts {
        raw_exact: Tally(raw_exact),
        documented: Tally(documented),
        contract_only: Tally(contract_only),
    }
}

fn changed_scope() -> Input {
    fixtures::project_scope(json!(["vendor", "src"]), "file", None)
}
fn changed_entry() -> Input {
    fixtures::project_scope(json!(["src"]), "missing", None)
}
fn stale_row() -> Input {
    fixtures::project_scope(json!(["src"]), "file", Some("stale-identity"))
}
fn bound_scope() -> Input {
    fixtures::project_scope(json!(["src"]), "file", None)
}
fn cohort_on_both() -> Input {
    let cohort = || Some(json!(["A001", "B002"]));
    fixtures::with_selected_rules(cohort(), [cohort(), cohort()])
}
fn cohort_narrowed_on_case() -> Input {
    let cohort = || Some(json!(["A001", "B002"]));
    fixtures::with_selected_rules(Some(json!(["A001"])), [cohort(), cohort()])
}
fn cohort_missing_on_reference_row() -> Input {
    let cohort = || Some(json!(["A001", "B002"]));
    fixtures::with_selected_rules(cohort(), [None, cohort()])
}
fn cohort_missing_on_case() -> Input {
    let cohort = || Some(json!(["A001", "B002"]));
    fixtures::with_selected_rules(None, [cohort(), cohort()])
}
fn declaration_on_both() -> Input {
    let declared = || Some(fixtures::observation_declaration("used λ\t\\source"));
    fixtures::with_rule_observations(declared(), [declared(), declared()])
}
fn declaration_changed_on_candidate_row() -> Input {
    let declared = || Some(fixtures::observation_declaration("used λ\t\\source"));
    fixtures::with_rule_observations(
        declared(),
        [declared(), Some(fixtures::observation_declaration("other"))],
    )
}
fn declaration_missing_on_reference_row() -> Input {
    let declared = || Some(fixtures::observation_declaration("used λ\t\\source"));
    fixtures::with_rule_observations(declared(), [None, declared()])
}
fn declaration_missing_on_case() -> Input {
    let declared = || Some(fixtures::observation_declaration("used λ\t\\source"));
    fixtures::with_rule_observations(None, [declared(), declared()])
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
        input: bound_scope,
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("an explicit cohort on the case and both rows is accepted"),
        word: Word::Succeeds,
        input: cohort_on_both,
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("observation declarations carried by both rows are accepted"),
        word: Word::Succeeds,
        input: declaration_on_both,
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("an unexpected candidate diagnostic is refused"),
        word: Word::Refuses,
        input: fixtures::candidate_with_unexpected_diagnostic,
        expect: Expect::Error(InventoryReceiptsError::DiagnosticsDiffer),
    },
    Case {
        name: CaseName("diagnostics that are not an array are refused"),
        word: Word::Refuses,
        input: fixtures::candidate_diagnostics_not_an_array,
        expect: Expect::Error(InventoryReceiptsError::DiagnosticsNotArray),
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
        input: || fixtures::without_case_key("file_metadata"),
        expect: Expect::Error(InventoryReceiptsError::Inputs(
            RegisteredInputsError::InputObservationsMissing,
        )),
    },
    Case {
        name: CaseName("a case without its metadata identity is refused"),
        word: Word::Refuses,
        input: || fixtures::without_case_key("file_metadata_sha256"),
        expect: Expect::Error(InventoryReceiptsError::FileMetadataDigestDiffers),
    },
    Case {
        name: CaseName("rows without the metadata identity are refused"),
        word: Word::Refuses,
        input: fixtures::rows_without_metadata_identity,
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a changed project scope no longer matches its registered identity"),
        word: Word::Refuses,
        input: changed_scope,
        expect: Expect::Error(InventoryReceiptsError::ProjectInputsDigestDiffers),
    },
    Case {
        name: CaseName("a changed project entry no longer matches its registered identity"),
        word: Word::Refuses,
        input: changed_entry,
        expect: Expect::Error(InventoryReceiptsError::ProjectInputsDigestDiffers),
    },
    Case {
        name: CaseName("a stale project identity on a row is refused"),
        word: Word::Refuses,
        input: stale_row,
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a cohort narrowed on the case is refused"),
        word: Word::Refuses,
        input: cohort_narrowed_on_case,
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a cohort missing on a row is refused"),
        word: Word::Refuses,
        input: cohort_missing_on_reference_row,
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a cohort missing on the case is refused"),
        word: Word::Refuses,
        input: cohort_missing_on_case,
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a changed observation declaration on a row is refused"),
        word: Word::Refuses,
        input: declaration_changed_on_candidate_row,
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("an observation declaration missing on a row is refused"),
        word: Word::Refuses,
        input: declaration_missing_on_reference_row,
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("an observation declaration missing on the case is refused"),
        word: Word::Refuses,
        input: declaration_missing_on_case,
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("a missing registered rule identity is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.0[0]["rule"] = Json::Null),
        expect: Expect::Error(InventoryReceiptsError::RegisteredRuleMissing),
    },
    Case {
        name: CaseName("a missing registered case identity is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.0[0]["name"] = Json::Null),
        expect: Expect::Error(InventoryReceiptsError::RegisteredCaseMissing),
    },
    Case {
        name: CaseName("a missing native rule identity is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.1[0]["rule"] = Json::Null),
        expect: Expect::Error(InventoryReceiptsError::NativeRuleMissing),
    },
    Case {
        name: CaseName("a missing native case identity is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.1[0]["name"] = Json::Null),
        expect: Expect::Error(InventoryReceiptsError::NativeCaseMissing),
    },
    Case {
        name: CaseName("a missing native side identity is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.1[0]["side"] = Json::Null),
        expect: Expect::Error(InventoryReceiptsError::NativeSideMissing),
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
        name: CaseName("an expected diagnostic without its rule is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["expected"] = json!([{"file": "input.txt", "start": 0, "end": 1}])
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiagnosticRuleMissing),
    },
    Case {
        name: CaseName("an expected diagnostic without its file is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["expected"] = json!([{"rule": "A001", "start": 0, "end": 1}])
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiagnosticFileMissing),
    },
    Case {
        name: CaseName("an expected diagnostic without its start is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["expected"] = json!([{"rule": "A001", "file": "input.txt", "end": 1}])
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiagnosticStartInvalid),
    },
    Case {
        name: CaseName("an expected diagnostic without its end is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["expected"] = json!([{"rule": "A001", "file": "input.txt", "start": 0}])
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiagnosticEndInvalid),
    },
    Case {
        name: CaseName("an expected diagnostic that ends before it starts is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["expected"] =
                    json!([{"rule": "A001", "file": "input.txt", "start": 2, "end": 1}])
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiagnosticSpanOrIdentityInvalid),
    },
    Case {
        name: CaseName("a discrepancy of an unknown basis is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["reference_discrepancy"] = json!({"basis": "unknown"})
            })
        },
        expect: Expect::Error(InventoryReceiptsError::UnknownDiscrepancyAuthority),
    },
    Case {
        name: CaseName("a process expectation under a complete-report contract is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["expectation_contract"] = json!("complete-report");
                d.0[0]["reference_discrepancy"] =
                    json!({"basis": "language-defect", "process": {"kind": "process-failure"}});
            })
        },
        expect: Expect::Error(InventoryReceiptsError::ProcessExpectationNotComplete),
    },
    Case {
        name: CaseName("an intended expectation that differs from the registered one is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(
                fixtures::passing,
                |d| d.0[0]["reference_discrepancy"] = json!({"basis": "language-defect", "intended_expected": [{"rule": "A001", "file": "input.txt", "start": 0, "end": 1}]}),
            )
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyIntentionDiffers),
    },
    Case {
        name: CaseName("a rule context that differs from the registration is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0]["rule_context"] = json!({"configured": true})
            })
        },
        expect: Expect::Error(InventoryReceiptsError::RuleContextDiffers),
    },
    Case {
        name: CaseName("an observation schema that differs from the registration is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0]["observation_schema"] = json!("other")
            })
        },
        expect: Expect::Error(InventoryReceiptsError::ExpectationContractDiffers),
    },
    Case {
        name: CaseName("a side that did not complete is refused"),
        word: Word::Refuses,
        input: || fixtures::edited(fixtures::passing, |d| d.1[1]["complete"] = json!(false)),
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a documented discrepancy the registration never declared is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0]["documented_reference_discrepancy"] = json!(true)
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyAuthorityDiffers),
    },
    Case {
        name: CaseName("a process observation inside a diagnostic report is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0]["process_observation"] = json!({})
            })
        },
        expect: Expect::Error(InventoryReceiptsError::UnexpectedProcessObservation),
    },
    Case {
        name: CaseName("a candidate that does not account for every pair is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[1]["coverage"]["selected_rule_file_pairs"] = json!(2)
            })
        },
        expect: Expect::Error(InventoryReceiptsError::CandidateNotAccounted),
    },
    Case {
        name: CaseName("a project context receipt in an isolated run is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0]["context_manifest"] = json!({"mode": "listed-disk"})
            })
        },
        expect: Expect::Error(InventoryReceiptsError::UnexpectedProjectContext),
    },
    Case {
        name: CaseName("a project context receipt that no longer matches its identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0]["project_context_mode"] = json!("listed-disk");
                d.0[0]["context_identity"] =
                    json!({"mode": "listed-disk", "logical_root": "root", "manifest_hash": "hash"});
            })
        },
        expect: Expect::Error(InventoryReceiptsError::StaleProjectContext),
    },
    Case {
        name: CaseName("a discrepancy bound to other file metadata is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                d.0[0]["reference_discrepancy"]["identity"]["file-metadata-sha256"] = json!("other")
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyMetadataDiffers),
    },
    Case {
        name: CaseName("a discrepancy validation that the sides do not share is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                d.1[1]["discrepancy_validation"] = Json::Null
            })
        },
        expect: Expect::Error(InventoryReceiptsError::DiscrepancyValidationMissing),
    },
    Case {
        name: CaseName("a discrepancy basis the sides did not verify is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::independently_bound_discrepancy, |d| {
                d.0[0]["reference_discrepancy"]["basis"] = json!("tool-contract-difference");
                for row in &mut d.1 {
                    row["documented_reference_discrepancy"] = json!(false);
                    row["documented_tool_contract_difference"] = json!(true);
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
                    row["discrepancy_validation"]["actual_executable_sha256"] = json!("");
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
                d.0[0]["reference_discrepancy"]["approved_executable_sha256"] =
                    json!(["another-pin"])
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
