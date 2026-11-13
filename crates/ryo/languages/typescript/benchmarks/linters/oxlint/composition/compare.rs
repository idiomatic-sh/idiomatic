//! The parity command: the pinned oxlint and this worker over every project of the language's benchmark, compared rule
//! by rule through the shared corpus runner.
use super::certification::{StagedSuite, candidate_linter, stage};
use super::commands;
use super::constants;
use super::locations::directory;
use ryo_benchmark::application::project_request::{service::request, types::ProjectCorpus};
use ryo_benchmark::domain::command_line::types::BenchmarkArgument;
use ryo_benchmark::domain::measurements::types::{LanguageTitle, corpus_shape::value::CorpusShape};
use ryo_benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark::inbound::use_cases::compare_corpus::{
    interface::CompareCorpusUseCase as _,
    types::{AcceptedDifferencesFile, CorpusParity},
    use_case::CompareCorpusUseCaseImpl,
};
use ryo_benchmark::outbound::discovery::interface::DiscoveryPort;
use ryo_benchmark::outbound::evidence::interface::EvidencePort;
use ryo_benchmark::outbound::linter::interface::LinterPort;
use ryo_benchmark::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark::outbound::workspace::interface::WorkspacePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ContestantName;
use ryo_benchmark_contracts::domain::requests::types::CohortSelection;
use ryo_benchmark_contracts::domain::workloads::types::{Project, Tool};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// Composition is not DRY: this port wiring repeats in each linter's composition by the owner's rule; only pure values are shared.
/// A stream whose destination the composition moves from one project's per-rule file to the next.
#[derive(Clone, Default)]
struct ProjectStream(Arc<Mutex<Option<Box<dyn Write + Send>>>>);

impl Write for ProjectStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.0.lock().map_err(|_poisoned| std::io::Error::other(constants::POISONED_STREAM))?.as_mut() {
            Some(stream) => stream.write(bytes),
            None => Ok(bytes.len()),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self.0.lock().map_err(|_poisoned| std::io::Error::other(constants::POISONED_STREAM))?.as_mut() {
            Some(stream) => stream.flush(),
            None => Ok(()),
        }
    }
}

/// The documents the evidence boundary writes, kept so that only its last, which holds every project, reaches stdout.
#[derive(Clone, Default)]
struct Documents(Arc<Mutex<Vec<u8>>>);

impl Write for Documents {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_poisoned| std::io::Error::other(constants::POISONED_STREAM))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Documents {
    /// The last line written, with its newline.
    fn last(&self) -> std::io::Result<Vec<u8>> {
        let written = self
            .0
            .lock()
            .map_err(|_poisoned| std::io::Error::other(constants::POISONED_STREAM))?;
        let body = written.strip_suffix(b"\n").unwrap_or(&written);
        let start = body.iter().rposition(|byte| *byte == b'\n').map_or(0, |index| index + 1);
        Ok(written.get(start..).unwrap_or_default().to_vec())
    }
}

/// What every project of one parity run is compared with.
struct Contestants {
    reference: Arc<super::super::adapter::LinterOxlintAdapter>,
    candidate: Arc<dyn LinterPort>,
    discovery: Arc<dyn DiscoveryPort>,
    workspace: Arc<dyn WorkspacePort>,
    evidence: Arc<dyn EvidencePort>,
    per_rule: ProjectStream,
    tool: Tool,
    output: PathBuf,
}

/// Compare the authored rules of the committed profile over every project of the language's benchmark: the pinned
/// oxlint against this worker, each project's differences judged by its accepted-differences file. The metrics of every
/// project are one document on stdout; each project's per-rule document is written to `<output>/<project>/per-rule.json`.
pub(super) fn parity(
    args: &mut dyn Iterator<Item = BenchmarkArgument>,
    observations: &mut dyn std::io::Write,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<(), Error> {
    let mut paths = args.map(|argument| PathBuf::from(argument.0));
    let (Some(oxlint), Some(output), None) = (paths.next(), paths.next(), paths.next()) else {
        return Err(Error::Invalid(constants::PARITY_ARGUMENTS.into()));
    };
    let oxlint = oxlint.canonicalize()?;
    let output = suite_source.ensure_directory(ArtifactPath(&output))?;
    let mut tool = suite_source.profile(ArtifactPath(&directory()))?;
    let expected = commands::reference(oxlint.clone(), tool.clone()).version_line(&tool);
    suite_source.verify_tool(ArtifactPath(&oxlint), &tool, &expected)?;
    let audit = ryo_benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(
        Arc::clone(suite_source),
    )
    .audit(&directory(), &tool)?;
    tool.defaults = audit.authored().to_vec();
    let StagedSuite {
        scratch,
        profile,
        workload,
    } = stage(suite_source, &output, audit.authored())?;
    let candidate_executable = scratch.join(constants::CANDIDATE_EXECUTABLE);
    suite_source.copy_file(
        ArtifactPath(&std::env::current_exe()?),
        ArtifactPath(&candidate_executable),
    )?;
    let documents = Documents::default();
    let per_rule = ProjectStream::default();
    let contestants = Contestants {
        reference: Arc::new(commands::reference(oxlint.clone(), tool.clone())),
        candidate: candidate_linter(
            &tool,
            candidate_executable,
            profile.join(constants::RULES_DIRECTORY),
        ),
        discovery: Arc::new(ryo_benchmark::outbound::discovery::factory::listing(
            oxlint,
            crate::adapter::LinterOxlintAdapter::discovery_arguments(&scratch)?,
        )),
        workspace: Arc::new(ryo_benchmark::outbound::workspace::factory::git(
            output.join(constants::PROJECTS_DIRECTORY),
        )),
        evidence: Arc::new(ryo_benchmark::outbound::evidence::factory::bmf(
            Box::new(documents.clone()),
            Box::new(per_rule.clone()),
        )),
        per_rule,
        tool,
        output,
    };
    let suite = suite_source.projects(ArtifactPath(&workload))?;
    CorpusShape {
        language: &constants::TYPESCRIPT.into(),
        display: LanguageTitle(constants::LANGUAGE_DISPLAY),
    }
    .admit(&suite)?;
    let mut failures = Vec::new();
    for project in &suite.projects {
        if let Err(error) = compare(&contestants, suite_source, project, observations) {
            writeln!(observations, "{}: parity failed: {error}", project.name)?;
            failures.push(project.name.clone());
        }
    }
    std::io::stdout().write_all(&documents.last()?)?;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!("parity failed: {failures:?}")))
    }
}

/// Compare one project: discover its sources with the reference, build its request, and judge it by its accepted differences.
fn compare(
    contestants: &Contestants,
    suite_source: &Arc<dyn SuitePort>,
    project: &Project,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let workspace = contestants.workspace.prepare(project)?;
    let files = contestants
        .discovery
        .files(workspace.root())?
        .into_iter()
        .filter(|file| constants::SOURCE_SUFFIXES.iter().any(|suffix| file.0.ends_with(suffix)))
        .collect::<Vec<_>>();
    let tracked = contestants
        .workspace
        .tracked_files(&workspace)?
        .0
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let project_output = contestants.output.join(&project.name.0);
    let _directory = suite_source.ensure_directory(ArtifactPath(&project_output))?;
    let configured = contestants.tool.defaults.clone();
    let cohort = CohortSelection::Explicit(configured.clone());
    let run = request(
        &ryo_benchmark::outbound::context::factory::local(),
        &ProjectCorpus {
            cohort: &cohort,
            rule_context: &contestants.tool.rule_context,
            search_roots: &contestants.tool.project_search_roots,
            context_mode: contestants.tool.project_context_mode,
        },
        workspace.root(),
        &files,
        &tracked,
        project_output.join(constants::PARITY_STATE),
    )?;
    *contestants
        .per_rule
        .0
        .lock()
        .map_err(|_poisoned| Error::Invalid(constants::POISONED_STREAM.into()))? = Some(
        suite_source.create_text(ArtifactPath(&project_output.join(constants::PER_RULE)))?,
    );
    let accepted = accepted_file(&project.name.0);
    let name = format!("{}/{}", project.name, constants::REFERENCE_NAME);
    let syntax = ryo::linter::RuleId::try_from(crate::constants::INVALID_SYNTAX)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    CompareCorpusUseCaseImpl::new(
        Arc::<super::super::adapter::LinterOxlintAdapter>::clone(&contestants.reference),
        Arc::clone(&contestants.candidate),
        Arc::new(ryo_benchmark::outbound::execution::factory::process()),
        Arc::new(ryo_benchmark::outbound::context::factory::local()),
        Arc::clone(suite_source),
        Arc::clone(&contestants.evidence),
    )
    .compare_corpus(
        &CorpusParity {
            request: &run,
            configured: &configured,
            accepted: if accepted.is_file() {
                AcceptedDifferencesFile::Declared(&accepted)
            } else {
                AcceptedDifferencesFile::Absent
            },
            reference: ContestantName(&name),
            syntax: &syntax,
        },
        observations,
    )
}

/// The differences accepted for a project: `profiles/evidence/<project>-accepted-differences.json` of this bench.
fn accepted_file(project: &str) -> PathBuf {
    directory()
        .join(constants::EVIDENCE_DIRECTORY)
        .join(format!("{project}{}", constants::ACCEPTED_SUFFIX))
}
