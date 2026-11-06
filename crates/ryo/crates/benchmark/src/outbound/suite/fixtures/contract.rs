//! Physical artifact observations shared by suite adapters; audit policy stays in application.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactSet},
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
    Invocation,
}
type Input = Operation;
type Value = usize;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Unexpected,
    MissingArtifact,
    Malformed,
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
        name: CaseName("a record without a process identity is refused"),
        word: Word::Refuses,
        input: || Operation::Invocation,
        expect: Expect::Error(Failure::Malformed),
    },
];

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
    let subject =
        |operation| {
            let missing = temporary.path().join("missing");
            let destination = temporary.path().join("destination");
            let result = match operation {
            Operation::Profile => adapter.profile(ArtifactPath(&missing)).map(|_| 0),
            Operation::Projects => adapter.projects(ArtifactPath(&missing)).map(|_| 0),
            Operation::Verify => adapter
                .verify_tool(ArtifactPath(&missing), &tool)
                .map(|()| 0),
            Operation::Fingerprint => adapter.fingerprint(ArtifactSet(&[missing])).map(|_| 0),
            Operation::Digest => adapter.digest(ArtifactPath(&missing)).map(|_| 0),
            Operation::Stage => adapter
                .stage(
                    ArtifactPath(&missing),
                    ArtifactPath(&destination),
                    &[],
                    &Default::default(),
                    ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
                )
                .map(|()| 0),
            Operation::StageProjects => adapter
                .stage_projects(ArtifactPath(&missing), ArtifactPath(&destination))
                .map(|()| 0),
            Operation::RuleIds => adapter
                .rule_ids(ArtifactPath(&temporary.path().join("rules")))
                .map(|ids| ids.len()),
            Operation::RuleQuery => adapter
                .rule_query(ArtifactPath(&temporary.path().join("rules")), &tool.defaults[0])
                .map(|query| usize::from(query.is_some())),
            Operation::Invocation => adapter.decode_invocation(b"{}").map(|_| 0),
            Operation::RuleCases => adapter
                .rule_cases(ArtifactPath(&temporary.path().join("rules")), &tool.defaults[0])
                .map(|cases| cases.cases.len()),
        };
            result.map_err(|error| match error {
                Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Failure::MissingArtifact
                }
                Error::Invalid(_) => Failure::Malformed,
                _ => Failure::Unexpected,
            })
        };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
