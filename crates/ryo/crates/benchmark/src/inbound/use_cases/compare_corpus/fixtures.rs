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
        Arc::new(crate::outbound::discovery::fixtures::fake::DiscoveryFake { files: Vec::new() }),
        Arc::new(crate::outbound::workspace::fixtures::fake::WorkspaceFake),
    )
}

/// A linter that reports only the findings of the files a request names, and that leaves `unanswered` unanswered: a run
/// naming it fails as an incomplete candidate does.
#[derive(Debug)]
pub struct FilesLinter {
    /// What it reports over every file.
    pub inner: LinterFake,
    /// Whether it answers every file it is given.
    pub answers: Answers,
}

/// Which files a `FilesLinter` answers.
#[derive(Debug)]
pub enum Answers {
    /// Every file it is given.
    Every,
    /// Every file but this one, whose checks it leaves unanswered.
    AllBut(RelativePath),
}

impl crate::outbound::linter::interface::LinterPort for FilesLinter {
    fn rule_configuration(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        self.inner.rule_configuration()
    }
    fn prepare(
        &self,
        request: &RunRequest,
    ) -> Result<
        ryo_benchmark_contracts::domain::measurements::types::Invocation,
        ryo_benchmark_contracts::domain::reports::errors::ReportsError,
    > {
        self.inner.prepare(request)
    }
    fn decode(
        &self,
        request: &RunRequest,
        output: ryo_benchmark_contracts::domain::measurements::types::ProcessOutput,
    ) -> Result<
        ryo_benchmark_contracts::domain::reports::types::RunReport,
        ryo_benchmark_contracts::domain::reports::errors::ReportsError,
    > {
        if let Answers::AllBut(file) = &self.answers
            && request.files.contains(file)
        {
            return Err(
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(format!(
                    "incomplete candidate: {} is read only in part",
                    file.0
                )),
            );
        }
        let mut report = self.inner.decode(request, output)?;
        report
            .diagnostics
            .retain(|finding| request.files.contains(&finding.file));
        Ok(report)
    }
    fn settings_text(
        &self,
        normalized: &ryo_benchmark_contracts::domain::measurements::types::Invocation,
        case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
        profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> Result<
        crate::outbound::linter::types::CanonicalText,
        ryo_benchmark_contracts::domain::reports::errors::ReportsError,
    > {
        self.inner.settings_text(normalized, case, profile)
    }
    fn rule_context_text(
        &self,
        prepared: &ryo_benchmark_contracts::domain::context::types::PreparedRuleContext,
    ) -> crate::outbound::linter::types::CanonicalText {
        self.inner.rule_context_text(prepared)
    }
    fn version_line(
        &self,
        tool: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    ) -> ryo_benchmark_contracts::domain::workloads::types::version_line::value::VersionLine {
        self.inner.version_line(tool)
    }
}

/// A checkout of one project at a fixed directory, tracking the files it names.
#[derive(Debug)]
pub struct ProjectWorkspace {
    /// The checkout's root.
    pub root: std::path::PathBuf,
    /// The files the project tracks.
    pub tracked: Vec<String>,
}

impl crate::outbound::workspace::interface::WorkspacePort for ProjectWorkspace {
    fn prepare(
        &self,
        project: &ryo_benchmark_contracts::domain::workloads::types::Project,
    ) -> Result<
        crate::outbound::workspace::interface::Workspace,
        ryo_benchmark_contracts::domain::reports::errors::ReportsError,
    > {
        Ok(crate::outbound::workspace::interface::Workspace {
            root: self.root.clone(),
            revision: crate::outbound::workspace::types::CommitRevision(project.revision.0.clone()),
            applied: None,
        })
    }
    fn tracked_files(
        &self,
        _: &crate::outbound::workspace::interface::Workspace,
    ) -> Result<
        crate::outbound::workspace::types::TrackedFiles,
        ryo_benchmark_contracts::domain::reports::errors::ReportsError,
    > {
        Ok(crate::outbound::workspace::types::TrackedFiles(
            self.tracked.clone(),
        ))
    }
    fn apply_patch(
        &self,
        _: &mut crate::outbound::workspace::interface::Workspace,
        _: &std::path::Path,
    ) -> Result<(), ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        Ok(())
    }
    fn reset(
        &self,
        _: &mut crate::outbound::workspace::interface::Workspace,
    ) -> Result<(), ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        Ok(())
    }
}

/// The use case over a project of two sources, `bad.py` and `case.py`, in `root`: the reference finds `A` in `case.py` and
/// refuses to parse `bad.py`; the candidate finds the same `A` and leaves `bad.py` unanswered.
///
/// # Panics
/// Never: the rule identifiers are valid literals.
#[must_use]
pub fn projects_use_case(root: &std::path::Path) -> CompareCorpusUseCaseImpl {
    let bad = RelativePath("bad.py".into());
    let unparsed = BenchmarkFinding {
        file: bad.clone(),
        ..finding_of(syntax(), 0)
    };
    CompareCorpusUseCaseImpl::new(
        Arc::new(FilesLinter {
            inner: LinterFake {
                diagnostics: vec![finding(0), unparsed],
                arguments: Vec::new(),
            },
            answers: Answers::Every,
        }),
        Arc::new(FilesLinter {
            inner: LinterFake {
                diagnostics: vec![finding(0)],
                arguments: Vec::new(),
            },
            answers: Answers::AllBut(bad.clone()),
        }),
        Arc::new(ExecutionFake {
            status: 0,
            ..ExecutionFake::default()
        }),
        Arc::new(ContextFake),
        Arc::new(SuiteFake::default()),
        Arc::new(EvidenceFake),
        Arc::new(crate::outbound::discovery::fixtures::fake::DiscoveryFake {
            files: vec![bad, RelativePath("case.py".into())],
        }),
        Arc::new(ProjectWorkspace {
            root: root.to_owned(),
            tracked: vec!["bad.py".into(), "case.py".into()],
        }),
    )
}
