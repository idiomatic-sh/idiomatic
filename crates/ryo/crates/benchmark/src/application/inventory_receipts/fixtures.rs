//! Registered cases and native observation rows the receipt validation is proven over.
use crate::domain::registered_inputs::service::project_inputs_digest;
use crate::outbound::inventory::types::case_document::{
    CaseContextIdentity, CaseDocument, Comparison, ContextIdentity, EntryState,
    ExpectationContract, FileMetadata, FileObservation, InventoryCompleteness, ProjectEntry,
    ProjectInventory, ProjectInventoryDocument, ProjectScope,
};
use crate::outbound::suite::types::{
    finding::{FindingOffset, Finding},
    observation_document::{
        ContextMode, ContextRun, FileMetadataDigest, IntentMatch, ObservationDocument,
        ProjectContextRecord, ProjectInputsDigest, ReferenceExpectation, RuleCohort,
        RuleContextRecord, RuleObservationsRecord, SourceIdentity, ToolContractRecord,
    },
    reference_discrepancy::{
        DiscrepancyIdentity, DiscrepancyValidation, ExpectationKind, ObservedExecutable,
        ReferenceDiscrepancy,
    },
    text::{
        CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, ContestantSide,
        YesNo,
    },
};
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::artifacts::types::{
    FileExecutable, FileMetadataEntry, digest::value::ArtifactDigest,
};
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ExecutionMetadata, RuleCoverage,
};
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;

/// The registered documents and the native rows observed for them.
pub type Documents = (Vec<CaseDocument>, Vec<ObservationDocument>);

fn sha(text: &str) -> Sha256Text {
    Sha256Text(text.into())
}

/// A finding at an exact span of the case file.
pub fn finding(rule: &str, file: &str, start: u64, end: u64) -> Finding {
    Finding {
        rule: RuleName(rule.into()),
        file: PathText(file.into()),
        start: FindingOffset(start),
        end: FindingOffset(end),
    }
}

/// A registered case with only its required fields.
pub fn registered_case(rule: &str, name: &str) -> CaseDocument {
    CaseDocument {
        rule: RuleName(rule.into()),
        name: CaseName(name.into()),
        case_sha256: sha("case-pin"),
        source_sha256: sha("source-pin"),
        files: vec![PathText("input.txt".into())],
        comparison: Comparison::Rule,
        expected: Vec::new(),
        project_context_mode: ContextMode::Isolated,
        context_identity: CaseContextIdentity::Isolated,
        reference_discrepancy: None,
        expectation_contract: ExpectationContract::Unwritten,
        rule_context: RuleContextRecord::Undeclared,
        rule_observations: RuleObservationsRecord::Undeclared,
        selected_rules: RuleCohort::Undeclared,
        file_observation: FileObservation::Undeclared,
        file_metadata: FileMetadata::Undeclared,
        file_metadata_sha256: None,
        project_search_roots: ProjectScope::Undeclared,
        given_project_inventory: ProjectInventory::Undeclared,
        project_inputs_sha256: ProjectInputsDigest::Undeclared,
    }
}

fn registered(name: &str) -> CaseDocument {
    registered_case("A001", name)
}

/// A complete, passing native observation of one side of one case.
pub fn observed_row(rule: &str, name: &str, side: ContestantSide) -> ObservationDocument {
    ObservationDocument {
        rule: RuleName(rule.into()),
        name: CaseName(name.into()),
        side,
        complete: YesNo::Yes,
        error: None,
        case_passed: YesNo::Yes,
        files: vec![PathText("input.txt".into())],
        source_sha256: SourceIdentity::Sealed(sha("source-pin")),
        file_metadata_sha256: FileMetadataDigest::Undeclared,
        project_inputs_sha256: ProjectInputsDigest::Undeclared,
        expected: Vec::new(),
        reference_expected: ReferenceExpectation::Derived(Vec::new()),
        diagnostics: Some(Vec::new()),
        skipped: Some(Vec::new()),
        observation_schema: SchemaName("benchmark-case/v2".into()),
        expectation_contract: ComparisonContract::Diagnostics,
        execution_metadata: Some(ExecutionMetadata::Complete),
        coverage: Some(RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        context_manifest: ContextRun::Isolated,
        project_context: ProjectContextRecord::NotCarried,
        rule_context: RuleContextRecord::Undeclared,
        rule_observations: RuleObservationsRecord::Undeclared,
        selected_rules: RuleCohort::Undeclared,
        actual_request_root: None,
        documented_reference_discrepancy: YesNo::No,
        documented_tool_contract_difference: ToolContractRecord::Unrecorded,
        discrepancy_validation: None,
        raw_parity_matches_intended: None,
        diagnostics_match_intended: IntentMatch::Unreported,
        process_observation: None,
    }
}

fn observed(name: &str) -> ObservationDocument {
    observed_row("A001", name, ContestantSide::Reference)
}

/// Both native sides of one named case.
fn sides(base: &ObservationDocument) -> Vec<ObservationDocument> {
    [ContestantSide::Reference, ContestantSide::Candidate]
        .into_iter()
        .map(|side| ObservationDocument {
            side,
            ..base.clone()
        })
        .collect()
}

/// One registered case with both complete, passing native sides.
pub fn passing() -> Documents {
    (vec![registered("case")], sides(&observed("case")))
}

/// The same registered case, observed with an extra diagnostic on the candidate side.
pub fn candidate_with_unexpected_diagnostic() -> Documents {
    let (cases, mut rows) = passing();
    rows[1].diagnostics = Some(vec![finding("A001", "input.txt", 0, 1)]);
    (cases, rows)
}

/// The candidate carries no diagnostics at all.
pub fn candidate_without_diagnostics() -> Documents {
    let (cases, mut rows) = passing();
    rows[1].diagnostics = None;
    (cases, rows)
}

fn first_and_unrelated() -> Vec<ObservationDocument> {
    let mut rows = sides(&observed("first"));
    rows.extend(sides(&observed("unrelated")));
    rows
}

/// The same case registered twice.
pub fn duplicate_registration() -> Documents {
    (
        vec![registered("first"), registered("first")],
        first_and_unrelated(),
    )
}

/// Registered identities that do not match the observed rows.
pub fn unregistered_observation() -> Documents {
    (
        vec![registered("second"), registered("first")],
        first_and_unrelated(),
    )
}

/// Every row observed twice.
pub fn duplicate_observation() -> Documents {
    let rows = sides(&observed("first"));
    let mut doubled = rows.clone();
    doubled.extend(rows);
    (vec![registered("second"), registered("first")], doubled)
}

fn documented_discrepancy(settings: Sha256Text) -> Documents {
    let finding = finding("A001", "input.txt", 0, 1);
    let mut case = registered("case");
    case.reference_discrepancy = Some(ReferenceDiscrepancy {
        basis: DiscrepancyBasis::LanguageDefect,
        identity: DiscrepancyIdentity {
            tool: ReasonText("tool".into()),
            version: ReasonText("1".into()),
            observed_executable_sha256: ObservedExecutable::NotObserved,
            profile: sha("profile-pin"),
            settings: settings.clone(),
            source: sha("source-pin"),
            reference_output: sha("reference-pin"),
            intended_output: sha("intended-pin"),
            file_metadata: None,
        },
        approved_executable_sha256: vec![Sha256Text("tool-pin".into())],
        reference_expected: ReferenceExpectation::Derived(vec![finding.clone()]),
        intended_expected: Vec::new(),
        reason: ReasonText("independent expectation".into()),
        process: None,
    });
    let mut base = observed("case");
    base.reference_expected = ReferenceExpectation::Derived(vec![finding.clone()]);
    base.discrepancy_validation = Some(DiscrepancyValidation {
        discrepancy_basis: DiscrepancyBasis::LanguageDefect,
        process_observation_group_sha256: None,
        reference_expectation_kind: ExpectationKind::Diagnostics,
        actual_executable_sha256: sha("tool-pin"),
        settings_sha256: settings,
        profile_sha256: sha("profile-pin"),
        source_sha256: sha("source-pin"),
        reference_output_sha256: sha("reference-pin"),
        intended_output_sha256: sha("intended-pin"),
        file_metadata_sha256: None,
    });
    base.documented_reference_discrepancy = YesNo::Yes;
    base.documented_tool_contract_difference = ToolContractRecord::Recorded(YesNo::No);
    let mut rows = sides(&base);
    rows[0].diagnostics = Some(vec![finding]);
    (vec![case], rows)
}

/// A reference difference bound to independent evidence on both sides.
pub fn independently_bound_discrepancy() -> Documents {
    documented_discrepancy(sha("settings-pin"))
}

/// The same difference whose settings identity is unbound.
pub fn unbound_discrepancy() -> Documents {
    documented_discrepancy(sha(""))
}

/// One file whose executable bit is declared and observed on both sides.
pub fn with_file_metadata(executable: bool) -> Documents {
    let flag = if executable {
        FileExecutable::Set
    } else {
        FileExecutable::Unset
    };
    let values = std::collections::BTreeMap::from([(RelativePath("input.txt".into()), flag)]);
    let digest = ArtifactDigest::file_metadata(Some(values.iter().map(|(path, executable)| {
        FileMetadataEntry {
            path,
            executable: *executable,
        }
    })))
    .ok()
    .flatten()
    .map(|value| Sha256Text(value.0));
    let mut case = registered("mode");
    case.file_observation = FileObservation::PosixExecutable;
    case.file_metadata = FileMetadata::Declared(std::collections::BTreeMap::from([(
        PathText("input.txt".into()),
        flag,
    )]));
    case.file_metadata_sha256 = digest.clone();
    let mut base = observed("mode");
    base.file_metadata_sha256 = FileMetadataDigest::from(digest);
    (vec![case], sides(&base))
}

/// The declaration of the other mode, kept against rows observed for this one.
pub fn declaring_the_other_mode() -> Documents {
    (with_file_metadata(true).0, with_file_metadata(false).1)
}

/// A case that never declared file metadata at all.
pub fn unspecified_legacy() -> Documents {
    let (mut cases, mut rows) = with_file_metadata(false);
    cases[0].file_observation = FileObservation::Undeclared;
    cases[0].file_metadata = FileMetadata::Undeclared;
    cases[0].file_metadata_sha256 = None;
    for row in &mut rows {
        row.file_metadata_sha256 = FileMetadataDigest::Undeclared;
    }
    (cases, rows)
}

/// The project scope and inventory are declared and their identity bound on all three documents.
pub fn project_scope(roots: &[&str], state: EntryState, row_identity: Option<&str>) -> Documents {
    let mut declared = with_file_metadata(false);
    let inventory = |state| ProjectInventoryDocument {
        completeness: InventoryCompleteness::Partial,
        entries: vec![ProjectEntry {
            path: PathText("src/localmod.py".into()),
            state,
        }],
    };
    declared.0[0].project_search_roots = ProjectScope::Roots(vec![PathText("src".into())]);
    declared.0[0].given_project_inventory =
        ProjectInventory::Declared(inventory(EntryState::File));
    let digest = project_inputs_digest(
        &crate::application::inventory_receipts::service::registered_inputs(&declared.0[0]),
    )
    .ok()
    .flatten()
    .map(|value| Sha256Text(value.0));
    declared.0[0].project_inputs_sha256 = ProjectInputsDigest::from(digest.clone());
    for row in &mut declared.1 {
        row.project_inputs_sha256 = ProjectInputsDigest::from(digest.clone());
    }
    declared.0[0].project_search_roots =
        ProjectScope::Roots(roots.iter().map(|root| PathText((*root).into())).collect());
    declared.0[0].given_project_inventory = ProjectInventory::Declared(inventory(state));
    if let Some(identity) = row_identity {
        declared.1[0].project_inputs_sha256 = ProjectInputsDigest::Declared(sha(identity));
    }
    declared
}

/// An explicit cohort declared on the case and on both rows.
pub fn with_selected_rules(
    case_cohort: Option<&[&str]>,
    row_cohorts: [Option<&[&str]>; 2],
) -> Documents {
    let names = |cohort: Option<&[&str]>| {
        cohort.map_or(RuleCohort::Undeclared, |cohort| {
            RuleCohort::Declared(cohort.iter().map(|rule| RuleName((*rule).into())).collect())
        })
    };
    let (mut cases, mut rows) = with_file_metadata(false);
    cases[0].selected_rules = names(case_cohort);
    for (row, cohort) in rows.iter_mut().zip(row_cohorts) {
        row.selected_rules = names(cohort);
    }
    (cases, rows)
}

/// The observation declarations declared on the case and on both rows.
pub fn with_rule_observations(
    case_value: Option<&str>,
    row_values: [Option<&str>; 2],
) -> Documents {
    let declaration = |text: Option<&str>| {
        text.map_or(RuleObservationsRecord::Undeclared, |text| {
            RuleObservationsRecord::Declared(CanonicalJson(text.into()))
        })
    };
    let (mut cases, mut rows) = with_file_metadata(false);
    cases[0].rule_observations = declaration(case_value);
    for (row, value) in rows.iter_mut().zip(row_values) {
        row.rule_observations = declaration(value);
    }
    (cases, rows)
}

/// The documents `base` builds, with one edit applied.
pub fn edited(base: fn() -> Documents, edit: fn(&mut Documents)) -> Documents {
    let mut documents = base();
    edit(&mut documents);
    documents
}

/// A listed context identity the rows do not carry.
pub fn listed_context() -> ContextIdentity {
    ContextIdentity {
        mode: ContextMode::ListedDisk,
        logical_root: PathText("root".into()),
        manifest_hash: Sha256Text("hash".into()),
        present_paths: Vec::new(),
        missing_paths: Vec::new(),
    }
}
