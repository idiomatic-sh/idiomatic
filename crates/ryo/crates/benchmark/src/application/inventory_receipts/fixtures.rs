//! Registered cases and native observation rows the receipt validation is proven over.
use crate::domain::registered_inputs::service::project_inputs_digest;
use crate::outbound::inventory::types::{
    case_document::{
        CaseDocument, ContextIdentity, ContextMode, EntryState, FileObservation,
        InventoryCompleteness, ProjectEntry, ProjectInventoryDocument,
    },
    discrepancy::{DiscrepancyIdentity, DiscrepancyValidation, ReferenceDiscrepancy},
    finding::ByteOffset,
    finding::Finding,
    observation_document::ObservationDocument,
    text::{CanonicalJson, CaseName, PathText, RuleName, SchemaName, Sha256Text, Side, YesNo},
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

fn sha(text: &str) -> Option<Sha256Text> {
    Some(Sha256Text(text.into()))
}

/// A finding at an exact span of the case file.
pub fn finding(rule: &str, file: &str, start: u64, end: u64) -> Finding {
    Finding {
        rule: RuleName(rule.into()),
        file: PathText(file.into()),
        start: ByteOffset(start),
        end: ByteOffset(end),
    }
}

fn registered(name: &str) -> CaseDocument {
    CaseDocument {
        rule: Some(RuleName("A001".into())),
        name: Some(CaseName(name.into())),
        source_sha256: sha("source-pin"),
        files: vec![PathText("input.txt".into())],
        ..CaseDocument::default()
    }
}

fn observed(name: &str) -> ObservationDocument {
    ObservationDocument {
        rule: Some(RuleName("A001".into())),
        name: Some(CaseName(name.into())),
        complete: Some(YesNo::Yes),
        case_passed: Some(YesNo::Yes),
        source_sha256: sha("source-pin"),
        files: Some(vec![PathText("input.txt".into())]),
        expected: Some(Vec::new()),
        reference_expected: Some(Vec::new()),
        diagnostics: Some(Vec::new()),
        skipped: Some(Vec::new()),
        observation_schema: Some(SchemaName("benchmark-case/v2".into())),
        expectation_contract: Some(ComparisonContract::Diagnostics),
        execution_metadata: Some(ExecutionMetadata::Complete),
        coverage: Some(RuleCoverage {
            selected: 1,
            evaluated: 1,
            skipped: 0,
            unknown_or_unread: 0,
        }),
        ..ObservationDocument::default()
    }
}

/// Both native sides of one named case.
fn sides(base: &ObservationDocument) -> Vec<ObservationDocument> {
    [Side::Reference, Side::Candidate]
        .into_iter()
        .map(|side| ObservationDocument {
            side: Some(side),
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

fn documented_discrepancy(settings: Option<Sha256Text>) -> Documents {
    let finding = finding("A001", "input.txt", 0, 1);
    let mut case = registered("case");
    case.reference_discrepancy = Some(ReferenceDiscrepancy {
        basis: DiscrepancyBasis::LanguageDefect,
        identity: DiscrepancyIdentity {
            settings: settings.clone(),
            profile: sha("profile-pin"),
            source: sha("source-pin"),
            reference_output: sha("reference-pin"),
            intended_output: sha("intended-pin"),
            ..DiscrepancyIdentity::default()
        },
        approved_executable_sha256: vec![Sha256Text("tool-pin".into())],
        reference_expected: Some(vec![finding.clone()]),
        intended_expected: Vec::new(),
        reason: None,
        process: None,
    });
    let mut base = observed("case");
    base.reference_expected = Some(vec![finding.clone()]);
    base.discrepancy_validation = Some(DiscrepancyValidation {
        actual_executable_sha256: sha("tool-pin"),
        settings_sha256: settings,
        profile_sha256: sha("profile-pin"),
        source_sha256: sha("source-pin"),
        reference_output_sha256: sha("reference-pin"),
        intended_output_sha256: sha("intended-pin"),
        ..DiscrepancyValidation::default()
    });
    base.documented_reference_discrepancy = Some(YesNo::Yes);
    base.documented_tool_contract_difference = Some(YesNo::No);
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
    documented_discrepancy(None)
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
    case.file_observation = Some(FileObservation::PosixExecutable);
    case.file_metadata = Some(std::collections::BTreeMap::from([(
        PathText("input.txt".into()),
        flag,
    )]));
    case.file_metadata_sha256 = digest.clone();
    let mut base = observed("mode");
    base.file_metadata_sha256 = digest;
    (vec![case], sides(&base))
}

/// The declaration of the other mode, kept against rows observed for this one.
pub fn declaring_the_other_mode() -> Documents {
    (with_file_metadata(true).0, with_file_metadata(false).1)
}

/// A case that never declared file metadata at all.
pub fn unspecified_legacy() -> Documents {
    let (mut cases, mut rows) = with_file_metadata(false);
    cases[0].file_observation = None;
    cases[0].file_metadata = None;
    cases[0].file_metadata_sha256 = None;
    for row in &mut rows {
        row.file_metadata_sha256 = None;
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
    declared.0[0].project_search_roots = Some(vec![PathText("src".into())]);
    declared.0[0].given_project_inventory = Some(inventory(EntryState::File));
    let digest = project_inputs_digest(
        &crate::application::inventory_receipts::service::registered_inputs(&declared.0[0]),
    )
    .ok()
    .flatten()
    .map(|value| Sha256Text(value.0));
    declared.0[0].project_inputs_sha256 = digest.clone();
    for row in &mut declared.1 {
        row.project_inputs_sha256 = digest.clone();
    }
    declared.0[0].project_search_roots =
        Some(roots.iter().map(|root| PathText((*root).into())).collect());
    declared.0[0].given_project_inventory = Some(inventory(state));
    if let Some(identity) = row_identity {
        declared.1[0].project_inputs_sha256 = sha(identity);
    }
    declared
}

/// An explicit cohort declared on the case and on both rows.
pub fn with_selected_rules(
    case_cohort: Option<&[&str]>,
    row_cohorts: [Option<&[&str]>; 2],
) -> Documents {
    let names = |cohort: &[&str]| cohort.iter().map(|rule| RuleName((*rule).into())).collect();
    let (mut cases, mut rows) = with_file_metadata(false);
    cases[0].selected_rules = case_cohort.map(names);
    for (row, cohort) in rows.iter_mut().zip(row_cohorts) {
        row.selected_rules = cohort.map(names);
    }
    (cases, rows)
}

/// The observation declarations declared on the case and on both rows.
pub fn with_rule_observations(
    case_value: Option<&str>,
    row_values: [Option<&str>; 2],
) -> Documents {
    let declaration = |text: &str| CanonicalJson(text.into());
    let (mut cases, mut rows) = with_file_metadata(false);
    cases[0].rule_observations = case_value.map(declaration);
    for (row, value) in rows.iter_mut().zip(row_values) {
        row.rule_observations = value.map(declaration);
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
