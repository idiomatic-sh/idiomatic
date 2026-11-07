//! Physical artifact observations shared by suite adapters; audit policy stays in application.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::{
    interface::SuitePort,
    types::{
        ArtifactPath, ArtifactSet, ArtifactText, MaterializedContext,
        discrepancy::{CaseArtifacts, EvidenceArtifact, EvidenceRequirement},
        observation_document::ObservationDocument,
    },
};
use ryo_benchmark_contracts::domain::{
    reports::errors::ReportsError as Error, workloads::types::Tool,
};

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Profile,
    Projects,
    Verify,
    Fingerprint,
    Digest,
    Stage,
    StageProjects,
    RuleIds,
    RuleQuery,
    RuleCases,
    MetadataUndeclared,
    ReceiptFile,
    SourceDigest,
    ReadArtifacts,
    VerifyArtifacts,
    ReferenceWires,
    RecordAndRead,
    ReadMissingReceipt,
    ScratchAndDiscard,
    CopyMissing,
    EnsureDirectory,
    WriteAndSelect,
    SelectMissing,
    TotalSize,
    SealMissing,
    QueryModules,
    MaterializeIsolated,
    MaterializeInventory,
}
type Input = Operation;
type Value = usize;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Unexpected,
    MissingArtifact,
    Refused,
}
type Failure = Refusal;

pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing profile is refused"),
        word: Word::Refuses,
        input: || Operation::Profile,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing shared manifest is refused"),
        word: Word::Refuses,
        input: || Operation::Projects,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing executable is refused"),
        word: Word::Refuses,
        input: || Operation::Verify,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing fingerprint input is refused"),
        word: Word::Refuses,
        input: || Operation::Fingerprint,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing executable digest input is refused"),
        word: Word::Refuses,
        input: || Operation::Digest,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing rule staging input is refused"),
        word: Word::Refuses,
        input: || Operation::Stage,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("missing project staging input is refused"),
        word: Word::Refuses,
        input: || Operation::StageProjects,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("empty rule directory is an empty observation"),
        word: Word::Succeeds,
        input: || Operation::RuleIds,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("missing query remains absent"),
        word: Word::Succeeds,
        input: || Operation::RuleQuery,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("missing case declaration is refused"),
        word: Word::Refuses,
        input: || Operation::RuleCases,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("undeclared file metadata is accepted"),
        word: Word::Succeeds,
        input: || Operation::MetadataUndeclared,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("no case receipt destination is chosen by default"),
        word: Word::Succeeds,
        input: || Operation::ReceiptFile,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("sources of a missing root have no digest"),
        word: Word::Refuses,
        input: || Operation::SourceDigest,
        expect: Expect::Error(Failure::Refused),
    },
    Case {
        name: CaseName("no named evidence reads no bytes"),
        word: Word::Succeeds,
        input: || Operation::ReadArtifacts,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("evidence of a missing profile is refused"),
        word: Word::Refuses,
        input: || Operation::VerifyArtifacts,
        expect: Expect::Error(Failure::Refused),
    },
    Case {
        name: CaseName("a profile without rules declares no reference expectations"),
        word: Word::Succeeds,
        input: || Operation::ReferenceWires,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a recorded observation reads back"),
        word: Word::Succeeds,
        input: || Operation::RecordAndRead,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a missing receipt is refused"),
        word: Word::Refuses,
        input: || Operation::ReadMissingReceipt,
        expect: Expect::Error(Failure::Refused),
    },
    Case {
        name: CaseName("a scratch directory exists until it is discarded"),
        word: Word::Succeeds,
        input: || Operation::ScratchAndDiscard,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a missing file is not copied"),
        word: Word::Refuses,
        input: || Operation::CopyMissing,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("a directory and its parents are ensured"),
        word: Word::Succeeds,
        input: || Operation::EnsureDirectory,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("written names are read back as a selection"),
        word: Word::Succeeds,
        input: || Operation::WriteAndSelect,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a missing selection is refused"),
        word: Word::Refuses,
        input: || Operation::SelectMissing,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("the size of files is their bytes"),
        word: Word::Succeeds,
        input: || Operation::TotalSize,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a missing context cannot be sealed"),
        word: Word::Refuses,
        input: || Operation::SealMissing,
        expect: Expect::Error(Failure::Refused),
    },
    Case {
        name: CaseName("a profile without query modules verifies"),
        word: Word::Succeeds,
        input: || Operation::QueryModules,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("an escaping case source is not materialized"),
        word: Word::Refuses,
        input: || Operation::MaterializeIsolated,
        expect: Expect::Error(Failure::Refused),
    },
    Case {
        name: CaseName("a Given file entry without a staged file is refused"),
        word: Word::Refuses,
        input: || Operation::MaterializeInventory,
        expect: Expect::Error(Failure::Refused),
    },
];

/// A request that declares no file metadata.
fn request(root: &std::path::Path) -> ryo_benchmark_contracts::domain::requests::types::RunRequest {
    use ryo_benchmark_contracts::domain::requests::types::{
        CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration, RuleContextPreparation,
        RunRequest, SearchRoots, StagedSources,
    };
    RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: root.to_path_buf(),
        files: Vec::new(),
        state: root.to_path_buf(),
    }
}

/// A case whose only source path leaves the directory it is materialized in.
fn escaping_case() -> ryo_benchmark_contracts::domain::workloads::types::Case {
    use ryo_benchmark_contracts::domain::requests::types::{
        CohortSelection, FileMetadataDeclaration, InputDeclaration, SearchRoots,
    };
    use ryo_benchmark_contracts::domain::workloads::types::{
        Case, CaseComparison, ConfigurationOverride, ContextDeclaration, InventoryDeclaration,
        ReferenceBehavior, RequestedFiles,
    };
    Case {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: ConfigurationOverride::Inherited,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: InventoryDeclaration::Undeclared,
        requested_files: RequestedFiles::Legacy,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context: ContextDeclaration::Undeclared,
        comparison: CaseComparison::Rule,
        name: String::from("escape").into(),
        files: std::iter::once((
            ryo::linter::RelativePath("../escape.py".into()),
            ryo::linter::SourceText("x".into()),
        ))
        .collect(),
        expected: Vec::new(),
        reference_discrepancy: ReferenceBehavior::Agrees,
    }
}

/// A Given inventory that names a file nothing staged.
fn given_file() -> ryo::linter::GivenProjectInventory {
    ryo::linter::GivenProjectInventory {
        entries: vec![ryo::linter::ProjectEntryObservation {
            path: ryo::linter::RelativePath("absent.py".into()),
            state: ryo::linter::ProjectEntryState::File,
        }],
        ..Default::default()
    }
}

/// One complete observation row of a native side.
fn observation() -> ObservationDocument {
    use crate::outbound::suite::types::{
        observation_document::{
            ContextRun, FileMetadataDigest, IntentMatch, ProjectContextRecord,
            ProjectInputsDigest, ReferenceExpectation, RuleCohort, RuleContextRecord,
            RuleObservationsRecord, SourceIdentity, ToolContractRecord,
        },
        text::{CaseName, ContestantSide, RuleName, SchemaName, YesNo},
    };
    use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
    ObservationDocument {
        rule: RuleName("R1".into()),
        name: CaseName("one".into()),
        side: ContestantSide::Reference,
        complete: YesNo::Yes,
        error: None,
        case_passed: YesNo::Yes,
        files: Vec::new(),
        source_sha256: SourceIdentity::Unavailable,
        file_metadata_sha256: FileMetadataDigest::Undeclared,
        project_inputs_sha256: ProjectInputsDigest::Undeclared,
        expected: Vec::new(),
        reference_expected: ReferenceExpectation::Underivable,
        diagnostics: None,
        skipped: None,
        observation_schema: SchemaName("benchmark-case/v2".into()),
        expectation_contract: ComparisonContract::Diagnostics,
        execution_metadata: None,
        coverage: None,
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

/// Exercise actual physical observations; policy expectations are proved by the application tests.
pub fn check(adapter: &impl SuitePort) {
    let temporary = tempfile::tempdir().expect("suite fixture");
    std::fs::create_dir_all(temporary.path().join("rules")).expect("rules fixture");
    let tool = Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
        project_context_mode: Default::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![String::new().into()],
        deferred: Default::default(),
        pending: Default::default(),
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration: None,
        source_language: None,
        project_search_roots: None,
        defaults: vec!["fixture".try_into().expect("rule identifier")],
    };
    let subject = |operation| {
        let missing = temporary.path().join("missing");
        let destination = temporary.path().join("destination");
        let rules = temporary.path().join("rules");
        let classify = |error: Error| match error {
            Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Failure::MissingArtifact
            }
            _ => Failure::Unexpected,
        };
        let refused = |_error: &dyn std::fmt::Debug| Failure::Refused;
        match operation {
            Operation::Profile => adapter.profile(ArtifactPath(&missing)).map(|_| 0).map_err(classify),
            Operation::Projects => adapter.projects(ArtifactPath(&missing)).map(|_| 0).map_err(classify),
            Operation::Verify => adapter
                .verify_tool(ArtifactPath(&missing), &tool)
                .map(|()| 0)
                .map_err(classify),
            Operation::Fingerprint => adapter
                .fingerprint(ArtifactSet(&[missing]))
                .map(|_| 0)
                .map_err(classify),
            Operation::Digest => adapter.digest(ArtifactPath(&missing)).map(|_| 0).map_err(classify),
            Operation::Stage => adapter
                .stage(
                    ArtifactPath(&missing),
                    ArtifactPath(&destination),
                    &[],
                    &Default::default(),
                    ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
                )
                .map(|()| 0)
                .map_err(classify),
            Operation::StageProjects => adapter
                .stage_projects(ArtifactPath(&missing), ArtifactPath(&destination))
                .map(|()| 0)
                .map_err(classify),
            Operation::RuleIds => adapter
                .rule_ids(ArtifactPath(&rules))
                .map(|ids| ids.len())
                .map_err(classify),
            Operation::RuleQuery => adapter
                .rule_query(ArtifactPath(&rules), &tool.defaults[0])
                .map(|query| usize::from(query.is_some()))
                .map_err(classify),
            Operation::RuleCases => adapter
                .rule_cases(ArtifactPath(&rules), &tool.defaults[0])
                .map(|cases| cases.cases.len())
                .map_err(classify),
            Operation::MetadataUndeclared => adapter
                .validate_file_metadata(&request(temporary.path()))
                .map(|()| 0)
                .map_err(classify),
            Operation::ReceiptFile => Ok(usize::from(adapter.case_receipt_file().is_some())),
            Operation::SourceDigest => adapter
                .case_source_digest(ArtifactPath(&missing), ArtifactSet(&[]))
                .map(|_| 0)
                .map_err(|error| refused(&error)),
            Operation::ReadArtifacts => adapter
                .read_discrepancy_artifacts(ArtifactPath(temporary.path()), &[])
                .map(|bytes| bytes.len())
                .map_err(|error| refused(&error)),
            Operation::VerifyArtifacts => {
                let evidence = [EvidenceArtifact {
                    path: "evidence.txt".into(),
                    sha256: String::new(),
                    requirement: EvidenceRequirement::Language,
                }];
                adapter
                    .verify_discrepancy_artifacts(CaseArtifacts {
                        profile: &missing,
                        root: &missing,
                        files: &[],
                        source_sha256: "",
                        file_metadata_files: &[],
                        file_metadata_sha256: None,
                        evidence: &evidence,
                    })
                    .map(|()| 0)
                    .map_err(|error| refused(&error))
            }
            Operation::ReferenceWires => adapter
                .case_reference_wires(ArtifactPath(temporary.path()), &[])
                .map(|wires| wires.len())
                .map_err(|error| refused(&error)),
            Operation::RecordAndRead => {
                let receipt = temporary.path().join("receipts").join("one.jsonl");
                adapter
                    .record_case(ArtifactPath(&receipt), observation(), None)
                    .and_then(|()| adapter.case_observations(ArtifactPath(&receipt)))
                    .map(|rows| rows.len())
                    .map_err(|error| refused(&error))
            }
            Operation::ReadMissingReceipt => adapter
                .case_observations(ArtifactPath(&missing))
                .map(|rows| rows.len())
                .map_err(|error| refused(&error)),
            Operation::ScratchAndDiscard => adapter
                .scratch(ArtifactPath(temporary.path()), ArtifactText("contract-"))
                .and_then(|scratch| {
                    let existed = usize::from(scratch.0.is_dir());
                    adapter.discard(ArtifactPath(&scratch.0))?;
                    Ok(existed * usize::from(!scratch.0.exists()))
                })
                .map_err(classify),
            Operation::CopyMissing => adapter
                .copy_file(ArtifactPath(&missing), ArtifactPath(&destination))
                .map(|()| 0)
                .map_err(classify),
            Operation::EnsureDirectory => adapter
                .ensure_directory(ArtifactPath(&temporary.path().join("ensured").join("nested")))
                .map(|directory| usize::from(directory.is_dir()))
                .map_err(classify),
            Operation::WriteAndSelect => {
                let file = temporary.path().join("selection.json");
                adapter
                    .write_text(ArtifactPath(&file), ArtifactText("[\"one\",\"two\"]"))
                    .and_then(|()| adapter.selection(ArtifactPath(&file)))
                    .map(|names| names.len())
                    .map_err(classify)
            }
            Operation::SelectMissing => adapter
                .selection(ArtifactPath(&missing))
                .map(|names| names.len())
                .map_err(classify),
            Operation::TotalSize => {
                let file = temporary.path().join("size.bin");
                std::fs::write(&file, b"abc").expect("size fixture");
                adapter
                    .total_size(ArtifactSet(&[file]))
                    .map(|size| usize::try_from(size.0).unwrap_or_default())
                    .map_err(classify)
            }
            Operation::SealMissing => adapter
                .seal_context(ArtifactPath(&missing), &[], ArtifactSet(&[]))
                .map(|_| 0)
                .map_err(|error| refused(&error)),
            Operation::QueryModules => adapter
                .verify_query_modules(ArtifactPath(temporary.path()), &tool)
                .map(|()| 0)
                .map_err(classify),
            Operation::MaterializeIsolated => adapter
                .materialize_case(ArtifactPath(&destination), &escaping_case())
                .map(|case| usize::from(case.context == MaterializedContext::Isolated))
                .map_err(|error| refused(&error)),
            Operation::MaterializeInventory => adapter
                .materialize_inventory(ArtifactPath(&missing), &given_file())
                .map(|()| 0)
                .map_err(|error| refused(&error)),
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
