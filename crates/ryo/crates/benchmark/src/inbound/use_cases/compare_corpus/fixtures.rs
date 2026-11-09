//! One corpus of one source compared over the fakes of every boundary, the reference reporting one
//! finding and the candidate that finding and one more.
#![expect(
    clippy::expect_used,
    reason = "fixture directories, files and rule identifiers are test preconditions"
)]
use super::use_case::CompareCorpusUseCaseImpl;
use crate::outbound::{
    context::fixtures::fake::ContextFake, evidence::fixtures::fake::EvidenceFake,
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
    suite::fixtures::fake::SuiteFake,
};
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, SourceByteOffset};
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use std::sync::Arc;

/// What a comparison varies.
#[derive(Clone, Copy, Debug)]
pub struct Scenario {
    /// Whether the request selects the configured rule `A` rather than `B`.
    pub configured: bool,
    /// Whether the accepted differences name this corpus's digest.
    pub this_corpus: bool,
    /// The column of the extra finding of `A` the accepted differences accept, on line 1.
    pub accepted_column: usize,
}

/// One corpus of one source and the accepted differences written for it.
#[derive(Debug)]
pub struct Corpus {
    /// The directory holding the source and the accepted differences, removed on drop.
    pub directory: tempfile::TempDir,
    /// The request both contestants run.
    pub request: RunRequest,
    /// The accepted-differences file.
    pub accepted: std::path::PathBuf,
    /// The digest of the corpus the request reads.
    pub digest: ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest,
}

/// The configured rule.
fn rule() -> RuleId {
    RuleId::try_from("A").expect("fixture rule")
}

/// A one-byte finding of `A` in `case.py` at `start`.
fn finding(start: usize) -> BenchmarkFinding {
    BenchmarkFinding {
        rule: rule(),
        file: RelativePath("case.py".into()),
        start: SourceByteOffset(start),
        end: SourceByteOffset(start + 1),
    }
}

/// The rule set the reference's profile configures.
#[must_use]
pub fn configured() -> Vec<RuleId> {
    vec![rule()]
}

/// A corpus whose one source is `a b c`, selecting `A` (or `B`) and accepting an extra `A` on line 1.
#[must_use]
pub fn corpus(scenario: Scenario) -> Corpus {
    let directory = tempfile::tempdir().expect("fixture corpus");
    std::fs::write(directory.path().join("case.py"), "a b c\n").expect("fixture source");
    let request = RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Explicit(vec![if scenario.configured {
            rule()
        } else {
            RuleId::try_from("B").expect("fixture rule")
        }]),
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: directory.path().to_owned(),
        files: vec![RelativePath("case.py".into())],
        state: directory.path().join("state"),
    };
    let digest = crate::domain::corpus_discrepancy::service::corpus_digest(
        &request.files,
        &request.context_manifest,
    );
    let written = if scenario.this_corpus {
        digest.0.clone()
    } else {
        "0".repeat(64)
    };
    let accepted = directory.path().join("accepted.json");
    std::fs::write(
        &accepted,
        format!(
            r#"{{"description":"d","corpus":"{written}","differences":[{{"rule":"A","reason":"r","findings":[{{"kind":"extra","file":"case.py","line":1,"column":{}}}]}}]}}"#,
            scenario.accepted_column
        ),
    )
    .expect("fixture accepted differences");
    Corpus {
        directory,
        request,
        accepted,
        digest,
    }
}

/// The use case over fakes: the reference finds `A` at byte 0, the candidate at bytes 0 and 2 (line 1, column 3).
#[must_use]
pub fn use_case() -> CompareCorpusUseCaseImpl {
    CompareCorpusUseCaseImpl::new(
        Arc::new(LinterFake {
            diagnostics: vec![finding(0)],
            arguments: Vec::new(),
        }),
        Arc::new(LinterFake {
            diagnostics: vec![finding(0), finding(2)],
            arguments: Vec::new(),
        }),
        Arc::new(ExecutionFake::default()),
        Arc::new(ContextFake),
        Arc::new(SuiteFake::default()),
        Arc::new(EvidenceFake),
    )
}
