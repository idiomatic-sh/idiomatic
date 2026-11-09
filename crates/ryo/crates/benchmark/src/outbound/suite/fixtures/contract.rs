//! Physical artifact observations shared by suite adapters; audit policy stays in application.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use super::samples::{escaping_case, given_file, request};
use crate::outbound::suite::{
    interface::SuitePort,
    types::{
        ArtifactPath, ArtifactSet, ArtifactText, MaterializedContext,
        ArtifactMetadata, CaseArtifacts, EvidenceArtifact, EvidenceRequirement,
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
    SourceDigest,
    ReadArtifacts,
    VerifyArtifacts,
    ReferenceWires,
    ScratchAndDiscard,
    RetainedDirectory,
    CopyMissing,
    EnsureDirectory,
    WriteAndSelect,
    SelectMissing,
    CreateAndRead,
    AcceptedMissing,
    TotalSize,
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
        name: CaseName("a scratch directory exists until it is discarded"),
        word: Word::Succeeds,
        input: || Operation::ScratchAndDiscard,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a retained directory stays after its handle is gone"),
        word: Word::Succeeds,
        input: || Operation::RetainedDirectory,
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
        name: CaseName("text streamed into a created file reads back as source text"),
        word: Word::Succeeds,
        input: || Operation::CreateAndRead,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("missing accepted differences are refused"),
        word: Word::Refuses,
        input: || Operation::AcceptedMissing,
        expect: Expect::Error(Failure::MissingArtifact),
    },
    Case {
        name: CaseName("the size of files is their bytes"),
        word: Word::Succeeds,
        input: || Operation::TotalSize,
        expect: Expect::Value(3),
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

/// Exercise actual physical observations; policy expectations are proved by the application tests.
pub fn check(adapter: &impl SuitePort) {
    let temporary = tempfile::tempdir().expect("suite fixture");
    std::fs::create_dir_all(temporary.path().join("rules")).expect("rules fixture");
    let tool = Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Undeclared,
        file_observation: ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared,
        project_context_mode: Default::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![String::new().into()],
        deferred: Default::default(),
        pending: Default::default(),
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
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
                    sha256: ArtifactDigest(String::new()),
                    requirement: EvidenceRequirement::Language,
                }];
                adapter
                    .verify_discrepancy_artifacts(CaseArtifacts {
                        profile: &missing,
                        root: &missing,
                        files: &[],
                        source_sha256: &ArtifactDigest(String::new()),
                        file_metadata_files: &[],
                        file_metadata: ArtifactMetadata::Undeclared,
                        evidence: &evidence,
                    })
                    .map(|()| 0)
                    .map_err(|error| refused(&error))
            }
            Operation::ReferenceWires => adapter
                .case_claims(ArtifactPath(temporary.path()), &[])
                .map(|claims| claims.len())
                .map_err(|error| refused(&error)),
            Operation::ScratchAndDiscard => adapter
                .scoped_directory(ArtifactPath(temporary.path()), ArtifactText("contract-"))
                .map(|scratch| {
                    let path = scratch.path().to_owned();
                    let existed = usize::from(path.is_dir());
                    drop(scratch);
                    existed * usize::from(!path.exists())
                })
                .map_err(classify),
            Operation::RetainedDirectory => adapter
                .retained_directory(ArtifactPath(temporary.path()), ArtifactText("kept-"))
                .map(|kept| usize::from(kept.0.is_dir()))
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
            Operation::CreateAndRead => {
                let file = temporary.path().join("source.py");
                adapter.create_text(ArtifactPath(&file)).and_then(|mut stream| Ok(std::io::Write::write_all(&mut stream, b"abc")?))
                    .and_then(|()| adapter.source_text(ArtifactPath(&file))).map(|source| source.0.len()).map_err(classify)
            }
            Operation::AcceptedMissing => adapter.accepted_differences(ArtifactPath(&missing)).map(|accepted| accepted.rules.len()).map_err(classify),
            Operation::TotalSize => {
                let file = temporary.path().join("size.bin");
                std::fs::write(&file, b"abc").expect("size fixture");
                adapter
                    .total_size(ArtifactSet(&[file]))
                    .map(|size| usize::try_from(size.0).unwrap_or_default())
                    .map_err(classify)
            }
            Operation::QueryModules => adapter
                .verify_query_modules(ArtifactPath(temporary.path()), &tool)
                .map(|()| 0)
                .map_err(classify),
            Operation::MaterializeIsolated => adapter
                .materialize_case(
                    ArtifactPath(&destination),
                    &escaping_case(),
                    &MaterializedContext::Isolated,
                )
                .map(|case| usize::from(case.root.is_absolute()))
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
