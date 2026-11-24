//! A certified project whose edit changes one file, measured over its real sealed context: the candidate alone reports one
//! finding the project's accepted differences name, and, when asked, one more in every file the edit changed.
#![expect(
    clippy::expect_used,
    reason = "fixture rule identifiers, directories and files are test preconditions"
)]

use super::super::types::SourceSuffix;
use super::super::use_case::MeasureCorpusUseCaseImpl;
use super::{bench_of, project_of, with_certificate};
use crate::outbound::linter::fixtures::fake::LinterFake;
use crate::outbound::linter::interface::LinterPort;
use crate::outbound::linter::types::CanonicalText;
use crate::outbound::workspace::interface::{Workspace, WorkspacePort};
use crate::outbound::workspace::types::{CommitRevision, TrackedFiles};
use benchmark_contracts::domain::context::types::PreparedRuleContext;
use benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use benchmark_contracts::domain::measurements::types::{
    Invocation, ProcessOutput, ResidentBytes, ResourceObservation, Resources,
};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::{BenchmarkFinding, RunReport, SourceByteOffset};
use benchmark_contracts::domain::requests::types::{ContextSeal, RunRequest};
use benchmark_contracts::domain::workloads::types::version_line::value::VersionLine;
use benchmark_contracts::domain::workloads::types::{PreparedCase, Project, Tool};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What the edit appends to the file it changes.
const EDIT_MARK: &str = "edited\n";
/// The project's files: the accepted difference is in the first, the edit changes the second.
const FILES: [&str; 2] = ["a.fixture", "b.fixture"];
/// What each file holds before the edit, as `project_of` writes it.
const ORIGINAL: &str = "a\n";

/// A checkout at a fixed directory whose patch appends `EDIT_MARK` to one file and whose reset writes it back.
#[derive(Debug)]
struct EditingWorkspace {
    /// The checkout's root.
    root: PathBuf,
}
impl WorkspacePort for EditingWorkspace {
    fn prepare(&self, project: &Project) -> Result<Workspace, Error> {
        Ok(Workspace {
            root: self.root.clone(),
            revision: CommitRevision(project.revision.0.clone()),
            applied: None,
        })
    }
    fn tracked_files(&self, _: &Workspace) -> Result<TrackedFiles, Error> {
        Ok(TrackedFiles(FILES.map(str::to_owned).to_vec()))
    }
    fn apply_patch(&self, _: &mut Workspace, _: &Path) -> Result<(), Error> {
        std::fs::write(self.root.join(FILES[1]), format!("{ORIGINAL}{EDIT_MARK}"))?;
        Ok(())
    }
    fn reset(&self, _: &mut Workspace) -> Result<(), Error> {
        std::fs::write(self.root.join(FILES[1]), ORIGINAL)?;
        Ok(())
    }
}

/// The candidate: the fake's findings, and, when `marks_edits`, rule A at the head of every file the edit changed.
#[derive(Debug)]
struct MarkingCandidate {
    /// The findings it reports in every run.
    inner: LinterFake,
    /// Whether it reports a finding in every edited file.
    marks_edits: bool,
}
impl LinterPort for MarkingCandidate {
    fn rule_configuration(
        &self,
    ) -> benchmark_contracts::domain::workloads::types::RuleConfigurationSupport {
        self.inner.rule_configuration()
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        self.inner.prepare(request)
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        let mut report = self.inner.decode(request, output)?;
        if self.marks_edits {
            for file in &request.files {
                if std::fs::read_to_string(request.root.join(file.0.as_ref()))?.contains(EDIT_MARK)
                {
                    report.diagnostics.push(finding(file.0.as_ref()));
                }
            }
        }
        Ok(report)
    }
    fn settings_text(
        &self,
        normalized: &Invocation,
        case: &PreparedCase,
        profile: &Tool,
    ) -> Result<CanonicalText, Error> {
        self.inner.settings_text(normalized, case, profile)
    }
    fn rule_context_text(&self, prepared: &PreparedRuleContext) -> CanonicalText {
        self.inner.rule_context_text(prepared)
    }
    fn version_line(&self, tool: &Tool) -> VersionLine {
        self.inner.version_line(tool)
    }
}

/// Rule A over the first byte of `file`.
fn finding(file: &str) -> BenchmarkFinding {
    BenchmarkFinding {
        rule: ryo::linter::RuleId::try_from("A").expect("rule identifier"),
        file: ryo::linter::RelativePath(file.into()),
        start: SourceByteOffset(0),
        end: SourceByteOffset(1),
    }
}

/// Measure the certified project whose candidate alone reports rule A at the head of the first file, a difference its
/// accepted-differences file names for the corpus as it stands before the edit; when `marks_edits`, the candidate also reports
/// rule A in the file the edit changes, which nothing accepts.
///
/// # Errors
/// What the measurement refuses.
///
/// # Panics
/// When the fixture's directories or files cannot be written, or the corpus cannot be sealed.
pub fn edited_corpus(
    marks_edits: bool,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let root = project_of(&FILES);
    let cache = project_of(&[]);
    let bench = bench_of(&["A"]);
    let files: Vec<_> = FILES
        .iter()
        .map(|file| ryo::linter::RelativePath((*file).into()))
        .collect();
    let tracked: Vec<_> = FILES.iter().map(PathBuf::from).collect();
    let sealed = crate::outbound::context::interface::ContextPort::seal_existing(
        &crate::outbound::context::factory::local(),
        root.path(),
        &files,
        crate::outbound::context::types::CorpusFiles(&tracked),
    )
    .expect("the corpus sealed");
    let corpus = crate::domain::corpus_discrepancy::service::corpus_digest(
        &files,
        &ContextSeal::Sealed(sealed),
    );
    let evidence = bench.path().join("profiles/evidence");
    std::fs::create_dir_all(&evidence).expect("an evidence directory");
    std::fs::write(
        evidence.join("project-0-accepted-differences.json"),
        serde_json::json!({
            "description": "the candidate's one extra finding",
            "corpus": corpus.0,
            "differences": [{
                "rule": "A",
                "reason": "accepted for the fixture",
                "findings": [{"kind": "extra", "file": FILES[0], "line": 1, "column": 1}],
            }],
        })
        .to_string(),
    )
    .expect("accepted differences");
    let use_case = MeasureCorpusUseCaseImpl::new(
        Arc::new(LinterFake::default()),
        Arc::new(MarkingCandidate {
            inner: LinterFake {
                diagnostics: vec![finding(FILES[0])],
                // A candidate run leaves a store, whose size the measurement reads.
                store: crate::outbound::linter::fixtures::fake::KeptState::Store(vec![0; 10]),
                ..LinterFake::default()
            },
            marks_edits,
        }),
        Arc::new(crate::outbound::discovery::fixtures::fake::DiscoveryFake { files }),
        Arc::new(crate::outbound::execution::fixtures::fake::ExecutionFake {
            status: 0,
            resources: ResourceObservation::Collected(Resources {
                cpu: std::time::Duration::from_secs(1),
                peak_rss_bytes: ResidentBytes(1_048_576),
            }),
            ..crate::outbound::execution::fixtures::fake::ExecutionFake::default()
        }),
        Arc::new(crate::outbound::suite::factory::local()),
        Arc::new(crate::outbound::context::factory::local()),
        Arc::new(crate::outbound::evidence::fixtures::fake::EvidenceFake),
        Arc::new(EditingWorkspace {
            root: root.path().to_owned(),
        }),
        Arc::new(crate::outbound::host::fixtures::fake::HostFake::default()),
        Arc::new(crate::outbound::engine::fixtures::fake::EngineFake::default()),
        Arc::new(crate::outbound::process_meter::fixtures::fake::ProcessMeterFake::default()),
    );
    with_certificate(
        (bench.path(), cache.path()),
        (&[SourceSuffix(".fixture")], ProjectContextMode::ListedDisk),
        |corpus| {
            super::super::interface::MeasureCorpusUseCase::measure_corpus(
                &use_case,
                corpus,
                observations,
            )
        },
    )
}
