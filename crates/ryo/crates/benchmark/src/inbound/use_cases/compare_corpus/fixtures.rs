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

/// The findings outside the selected rule a comparison adds, at byte 4 (line 1, column 5).
#[derive(Clone, Copy, Debug)]
pub enum Outside {
    /// None.
    Nothing,
    /// The candidate reports the unselected rule `Z`.
    Unselected,
    /// Only the reference rejects the source's syntax.
    SyntaxOnReference,
    /// Both contestants reject the source's syntax at the same place.
    SyntaxOnBoth,
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

/// The rule both contestants report for a source their parser rejects.
///
/// # Panics
/// Never: the rule identifier is a valid literal.
#[must_use]
pub fn syntax() -> RuleId {
    RuleId::try_from("invalid-syntax").expect("fixture rule")
}

/// A one-byte finding of `A` in `case.py` at `start`.
fn finding(start: usize) -> BenchmarkFinding {
    finding_of(rule(), start)
}

/// A one-byte finding of `rule` in `case.py` at `start`.
fn finding_of(rule: RuleId, start: usize) -> BenchmarkFinding {
    BenchmarkFinding {
        rule,
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
///
/// # Panics
/// When the corpus directory, its source or its accepted differences cannot be written.
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
    use_case_with(Outside::Nothing)
}

/// The use case of `use_case`, with the findings outside the selected rule that `outside` adds.
///
/// # Panics
/// Never: the rule identifiers are valid literals.
#[must_use]
pub fn use_case_with(outside: Outside) -> CompareCorpusUseCaseImpl {
    let mut reference = vec![finding(0)];
    let mut candidate = vec![finding(0), finding(2)];
    match outside {
        Outside::Nothing => {}
        Outside::Unselected => {
            candidate.push(finding_of(RuleId::try_from("Z").expect("fixture rule"), 4));
        }
        Outside::SyntaxOnReference => reference.push(finding_of(syntax(), 4)),
        Outside::SyntaxOnBoth => {
            reference.push(finding_of(syntax(), 4));
            candidate.push(finding_of(syntax(), 4));
        }
    }
    CompareCorpusUseCaseImpl::new(
        Arc::new(LinterFake {
            diagnostics: reference,
            arguments: Vec::new(),
        }),
        Arc::new(LinterFake {
            diagnostics: candidate,
            arguments: Vec::new(),
        }),
        Arc::new(ExecutionFake {
            status: 0,
            ..ExecutionFake::default()
        }),
        Arc::new(ContextFake),
        Arc::new(SuiteFake::default()),
        Arc::new(EvidenceFake),
    )
}
