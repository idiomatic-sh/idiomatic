//! What the `compare_corpus` use case promises, against the fakes of the ports it calls, from its fixtures.
use super::fixtures::{
    Outside, Scenario, configured, corpus, projects_use_case, syntax, use_case_with,
};
use super::interface::CompareCorpusUseCase as _;
use super::types::{
    AcceptedDifferencesFile, AcceptedSuffix, Corpus, CorpusParity, ProjectsParity, SourceExtension,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::types::ContestantName;
use ryo_benchmark_contracts::domain::requests::types::{CohortSelection, SearchRoots};
use ryo_benchmark_contracts::domain::workloads::types::{
    Project, RuleContextProfile, project_name::value::ProjectName,
    project_revision::value::ProjectRevision, repository_url::value::RepositoryUrl,
};

/// What a row compares.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One built request under a scenario, with the findings outside the selected rule the contestants add.
    Request(Scenario, Outside),
    /// A project of two sources, one of which the reference refuses to parse and the candidate leaves unanswered.
    Projects,
}

type Input = Given;
/// What the run wrote.
type Value = &'static str;
/// The refusal, the corpus digest spelled `<digest>`, and what the run wrote before it.
type Failure = (&'static str, &'static str);

/// The corpus whose one candidate extra finding the accepted differences name.
const ACCEPTED: Scenario = Scenario {
    configured: true,
    this_corpus: true,
    accepted_column: 3,
};

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a difference the accepted differences name passes"),
        word: Word::Succeeds,
        input: || Given::Request(ACCEPTED, Outside::Nothing),
        expect: Expect::Value("fixture parity: 0 identical of 1 rules; 0 unaccepted\n"),
    },
    Case {
        name: CaseName("both contestants rejecting the same syntax is no difference"),
        word: Word::Succeeds,
        input: || Given::Request(ACCEPTED, Outside::SyntaxOnBoth),
        expect: Expect::Value("fixture parity: 0 identical of 1 rules; 0 unaccepted\n"),
    },
    Case {
        name: CaseName("a finding of an unselected rule is a problem"),
        word: Word::Refuses,
        input: || Given::Request(ACCEPTED, Outside::Unselected),
        expect: Expect::Error((
            "1 unaccepted parity differences",
            "fixture parity: 0 identical of 1 rules; 1 unaccepted\nZ: outside the selected rules, the candidate reports 1 and the reference 0\n",
        )),
    },
    Case {
        name: CaseName("syntax only one contestant rejects is a problem"),
        word: Word::Refuses,
        input: || Given::Request(ACCEPTED, Outside::SyntaxOnReference),
        expect: Expect::Error((
            "1 unaccepted parity differences",
            "fixture parity: 0 identical of 1 rules; 1 unaccepted\ninvalid-syntax: only the reference rejects case.py:1:5\n",
        )),
    },
    Case {
        name: CaseName("a difference at another place fails after reporting each problem"),
        word: Word::Refuses,
        input: || {
            Given::Request(
                Scenario {
                    configured: true,
                    this_corpus: true,
                    accepted_column: 1,
                },
                Outside::Nothing,
            )
        },
        expect: Expect::Error((
            "2 unaccepted parity differences",
            "fixture parity: 0 identical of 1 rules; 2 unaccepted\nA: extra case.py:1:3 not accepted\nA: accepted extra case.py:1:1 no longer occurs\n",
        )),
    },
    Case {
        name: CaseName(
            "a request selecting other than the configured rules is refused before either contestant runs",
        ),
        word: Word::Refuses,
        input: || {
            Given::Request(
                Scenario {
                    configured: false,
                    this_corpus: true,
                    accepted_column: 3,
                },
                Outside::Nothing,
            )
        },
        expect: Expect::Error((
            "parity requires the request to select exactly the configured rules: 1 configured, 1 selected; unselected [RuleId(\"A\")], unconfigured [RuleId(\"B\")]",
            "",
        )),
    },
    Case {
        name: CaseName(
            "accepted differences written for another corpus are refused before either contestant runs",
        ),
        word: Word::Refuses,
        input: || {
            Given::Request(
                Scenario {
                    configured: true,
                    this_corpus: false,
                    accepted_column: 3,
                },
                Outside::Nothing,
            )
        },
        expect: Expect::Error((
            "accepted differences are for corpus 0000000000000000000000000000000000000000000000000000000000000000, the request reads corpus <digest>",
            "",
        )),
    },
    Case {
        name: CaseName(
            "a source the reference refuses to parse is left out of parity and counted, as measure-valid leaves it out",
        ),
        word: Word::Succeeds,
        input: || Given::Projects,
        expect: Expect::Value(
            "p: excluded the sources the reference refuses to parse: 1 [bad.py]\np/fixture parity: 1 identical of 1 rules; 0 unaccepted\n",
        ),
    },
];

/// Compares the row's corpus.
fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::Request(scenario, outside) => request(scenario, outside),
        Given::Projects => projects(),
    }
}

/// Compares the fixture corpus under one scenario.
fn request(scenario: Scenario, outside: Outside) -> Result<Value, Failure> {
    let corpus = corpus(scenario);
    let mut observations = Vec::new();
    let result = use_case_with(outside).compare_corpus(
        &CorpusParity {
            corpus: Corpus::Request {
                request: &corpus.request,
                accepted: AcceptedDifferencesFile::Declared(&corpus.accepted),
            },
            configured: &configured(),
            reference: ContestantName("fixture"),
            syntax: &syntax(),
        },
        &mut observations,
    );
    answer(result, &observations, &corpus.digest.0)
}

/// Compares the fixture project of two sources, one the reference refuses to parse.
fn projects() -> Result<Value, Failure> {
    let refusal = |error: std::io::Error| -> Failure { (error.to_string().leak(), "") };
    let directory = tempfile::tempdir().map_err(refusal)?;
    for (file, text) in [("bad.py", "a (\n"), ("case.py", "a b c\n")] {
        std::fs::write(directory.path().join(file), text).map_err(refusal)?;
    }
    let project = Project {
        name: ProjectName("p".into()),
        repository: RepositoryUrl("https://example.invalid/p.git".into()),
        revision: ProjectRevision("0".repeat(40)),
        patch: "p.patch".into(),
    };
    let cohort = CohortSelection::Explicit(configured());
    let mut observations = Vec::new();
    let result = projects_use_case(directory.path()).compare_corpus(
        &CorpusParity {
            corpus: Corpus::Projects(ProjectsParity {
                projects: std::slice::from_ref(&project),
                suffixes: &[SourceExtension(".py")],
                cohort: &cohort,
                rule_context: &RuleContextProfile::Undeclared,
                search_roots: &SearchRoots::Undeclared,
                context_mode: ProjectContextMode::Isolated,
                state: &directory.path().join("state"),
                accepted: directory.path(),
                accepted_suffix: AcceptedSuffix("-accepted-differences.json"),
            }),
            configured: &configured(),
            reference: ContestantName("fixture"),
            syntax: &syntax(),
        },
        &mut observations,
    );
    answer(result, &observations, "")
}

/// What the run wrote, or its refusal with the corpus digest `digest` spelled `<digest>` and what it wrote before it.
fn answer(
    result: Result<(), ryo_benchmark_contracts::domain::reports::errors::ReportsError>,
    observations: &[u8],
    digest: &str,
) -> Result<Value, Failure> {
    let observations: &'static str = String::from_utf8_lossy(observations).into_owned().leak();
    match result {
        Ok(()) => Ok(observations),
        Err(error) => Err((
            if digest.is_empty() {
                error.to_string()
            } else {
                error.to_string().replace(digest, "<digest>")
            }
            .leak(),
            observations,
        )),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
