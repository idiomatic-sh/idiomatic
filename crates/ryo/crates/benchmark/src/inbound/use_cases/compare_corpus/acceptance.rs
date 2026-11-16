//! The steps of the `compare_corpus` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::compare_corpus::{
    fixtures::{
        SUFFIXED_SOURCES, Scenario, configured, corpus, suffixed_use_case, syntax, use_case,
    },
    interface::CompareCorpusUseCase as _,
    types::{
        AcceptedDifferencesFile, AcceptedSuffix, Corpus, CorpusParity, ProjectsParity,
        SourceExtension,
    },
};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::reports::types::ContestantName;
use ryo_benchmark_contracts::domain::requests::types::{CohortSelection, SearchRoots};
use ryo_benchmark_contracts::domain::workloads::types::{
    Project, RuleContextProfile, project_name::value::ProjectName,
    project_revision::value::ProjectRevision, repository_url::value::RepositoryUrl,
};

#[then("compare_corpus passes a corpus whose one difference is accepted")]
fn accepted(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(compare(true).0.is_ok());
}

#[then("compare_corpus refuses a request that selects other than the configured rules")]
fn unconfigured(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let (result, observations) = compare(false);
    assert!(result.is_err() && observations.is_empty());
}

#[then(
    "compare_corpus compares a.ts and b.js and skips c.vue when the profile declares .ts .mts .cts .tsx .js .jsx .mjs .cjs"
)]
fn suffixed(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(compare_leaving("c.vue").is_ok());
    assert!(compare_leaving("a.ts").is_err());
    assert!(compare_leaving("b.js").is_err());
}

/// Compare the project of the suffixed sources under the TypeScript and JavaScript suffixes, the candidate leaving `unanswered`
/// unanswered: the comparison fails exactly when it compares that file.
fn compare_leaving(unanswered: &str) -> Result<(), ReportsError> {
    let directory =
        tempfile::tempdir().map_err(|error| ReportsError::Invalid(error.to_string()))?;
    for source in SUFFIXED_SOURCES {
        std::fs::write(directory.path().join(source), "a b c\n")
            .map_err(|error| ReportsError::Invalid(error.to_string()))?;
    }
    let project = Project {
        name: ProjectName("p".into()),
        repository: RepositoryUrl("https://example.invalid/p.git".into()),
        revision: ProjectRevision("0".repeat(40)),
        patch: "p.patch".into(),
    };
    let cohort = CohortSelection::Explicit(configured());
    let suffixes =
        [".ts", ".mts", ".cts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"].map(SourceExtension);
    suffixed_use_case(directory.path(), unanswered).compare_corpus(
        &CorpusParity {
            bench: std::path::Path::new("bench"),
            corpus: Corpus::Projects(ProjectsParity {
                projects: std::slice::from_ref(&project),
                suffixes: &suffixes,
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
        &mut Vec::new(),
    )
}

/// Compare the fixture corpus, the request selecting the configured rule or another, and what was written.
fn compare(configured_rule: bool) -> (Result<(), ReportsError>, Vec<u8>) {
    let corpus = corpus(Scenario {
        configured: configured_rule,
        this_corpus: true,
        accepted_column: 3,
    });
    let mut observations = Vec::new();
    let result = use_case().compare_corpus(
        &CorpusParity {
            bench: std::path::Path::new("bench"),
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
    (result, observations)
}
