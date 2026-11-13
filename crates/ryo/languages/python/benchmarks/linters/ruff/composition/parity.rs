//! The parity command: the pinned Ruff and this worker over every project of the language's benchmark, compared rule
//! by rule through the shared corpus runner. Every project's per-rule document is a line of `<output>/per-rule.json`.
use super::certification::{StagedSuite, candidate_linter, stage};
use super::commands;
use super::constants;
use super::locations::directory;
use ryo_benchmark::domain::command_line::types::BenchmarkArgument;
use ryo_benchmark::domain::measurements::types::{LanguageTitle, corpus_shape::value::CorpusShape};
use ryo_benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark::inbound::use_cases::compare_corpus::{
    interface::CompareCorpusUseCase as _,
    types::{AcceptedSuffix, Corpus, CorpusParity, ProjectsParity, SourceExtension},
    use_case::CompareCorpusUseCaseImpl,
};
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ContestantName;
use ryo_benchmark_contracts::domain::requests::types::CohortSelection;
use std::path::PathBuf;
use std::sync::Arc;

/// Compare the authored rules of the committed profile over every project of the language's benchmark: the pinned
/// Ruff against this worker, each project's differences judged by its accepted-differences file. The metrics of every
/// project are one document on stdout; each project's per-rule document is written to `<output>/<project>/per-rule.json`.
pub(super) fn parity(
    args: &mut dyn Iterator<Item = BenchmarkArgument>,
    observations: &mut dyn std::io::Write,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<(), Error> {
    let mut paths = args.map(|argument| PathBuf::from(argument.0));
    let (Some(ruff), Some(output), None) = (paths.next(), paths.next(), paths.next()) else {
        return Err(Error::Invalid(constants::PARITY_ARGUMENTS.into()));
    };
    let ruff = ruff.canonicalize()?;
    let output = suite_source.ensure_directory(ArtifactPath(&output))?;
    let mut tool = suite_source.profile(ArtifactPath(&directory()))?;
    let expected = commands::reference(ruff.clone(), tool.clone())?.version_line(&tool);
    suite_source.verify_tool(ArtifactPath(&ruff), &tool, &expected)?;
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
    let compare = CompareCorpusUseCaseImpl::new(
        Arc::new(commands::reference(ruff.clone(), tool.clone())?),
        candidate_linter(&tool, candidate_executable, profile.join(constants::RULES_DIRECTORY)),
        Arc::new(ryo_benchmark::outbound::execution::factory::process()),
        Arc::new(ryo_benchmark::outbound::context::factory::local()),
        Arc::clone(suite_source),
        Arc::new(ryo_benchmark::outbound::evidence::factory::bmf(
            Box::new(std::io::stdout()),
            suite_source.create_text(ArtifactPath(&output.join(constants::PER_RULE)))?,
        )),
        Arc::new(ryo_benchmark::outbound::discovery::factory::listing(
            ruff,
            crate::constants::DISCOVERY_ARGUMENTS
                .into_iter()
                .map(std::ffi::OsString::from)
                .collect(),
        )),
        Arc::new(ryo_benchmark::outbound::workspace::factory::git(
            output.join(constants::PROJECTS_DIRECTORY),
        )),
    );
    let suite = suite_source.projects(ArtifactPath(&workload))?;
    CorpusShape {
        language: &constants::PYTHON.into(),
        display: LanguageTitle(constants::LANGUAGE_DISPLAY),
    }
    .admit(&suite)?;
    let syntax = ryo::linter::RuleId::try_from(constants::INVALID_SYNTAX)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let result = compare.compare_corpus(
        &CorpusParity {
            corpus: Corpus::Projects(ProjectsParity {
                projects: &suite.projects,
                suffixes: &constants::SOURCE_SUFFIXES.map(SourceExtension),
                cohort: &CohortSelection::Explicit(tool.defaults.clone()),
                rule_context: &tool.rule_context,
                search_roots: &tool.project_search_roots,
                context_mode: tool.project_context_mode,
                state: &output.join(constants::PARITY_STATE),
                accepted: &directory().join(constants::EVIDENCE_DIRECTORY),
                accepted_suffix: AcceptedSuffix(constants::ACCEPTED_SUFFIX),
            }),
            configured: &tool.defaults,
            reference: ContestantName(constants::REFERENCE_NAME),
            syntax: &syntax,
        },
        observations,
    );
    drop(compare);
    result
}
