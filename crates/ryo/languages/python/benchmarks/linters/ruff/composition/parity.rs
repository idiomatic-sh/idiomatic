//! The parity command: pinned Ruff and a ryo worker over one staged request, compared rule by rule
//! through the shared corpus runner.
use super::commands;
use super::constants;
use super::locations::{default_scratch, directory};
use super::request_input::BenchmarkArgument;
use ryo_benchmark::inbound::use_cases::compare_corpus::{
    interface::CompareCorpusUseCase as _, types::CorpusParity, use_case::CompareCorpusUseCaseImpl,
};
use ryo_benchmark::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactText},
};
use ryo_benchmark::outbound::worker::{interface::WorkerPort as _, types::WorkerSource};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::ContestantName;
use ryo_benchmark_contracts::domain::requests::types::{
    FileMetadataDeclaration, RunRequest, StagedSources,
};
use std::{path::PathBuf, sync::Arc};

/// Compare the configured rules of the committed profile over a staged request: the pinned Ruff against
/// the given ryo worker, differences judged by the accepted-differences file, the metrics on stdout and
/// the per-rule document written to its own file.
pub(super) fn parity(
    args: &mut dyn Iterator<Item = BenchmarkArgument>,
    observations: &mut dyn std::io::Write,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<(), Error> {
    let mut paths = args.map(|argument| PathBuf::from(argument.0));
    let (Some(ruff), Some(worker), Some(staged), Some(accepted), Some(per_rule), None) = (
        paths.next(),
        paths.next(),
        paths.next(),
        paths.next(),
        paths.next(),
        paths.next(),
    ) else {
        return Err(Error::Invalid(constants::PARITY_ARGUMENTS.into()));
    };
    let (ruff, worker, staged, accepted) = (
        ruff.canonicalize()?,
        worker.canonicalize()?,
        staged.canonicalize()?,
        accepted.canonicalize()?,
    );
    let tool = suite_source.profile(ArtifactPath(&directory()))?;
    suite_source.verify_tool(ArtifactPath(&ruff), &tool)?;
    let configured = tool.defaults.clone();
    let staged_request = ryo_benchmark::outbound::worker::factory::json(
        WorkerSource::Stored(staged),
        Box::new(std::io::sink()),
    )
    .receive()?;
    let scratch = default_scratch(
        suite_source.as_ref(),
        ArtifactText(constants::PARITY_SCRATCH),
    )?;
    let request = RunRequest {
        performance: staged_request.performance,
        selected_rules: staged_request.selected_rules.clone(),
        rule_observations: staged_request.rule_observations.clone(),
        rule_context: staged_request.rule_context.clone(),
        project_search_roots: staged_request.project_search_roots.clone(),
        given_project_inventory: staged_request.given_project_inventory.clone(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: staged_request.context_manifest.clone(),
        root: staged_request.root.clone(),
        files: staged_request.files.clone(),
        state: scratch.path().to_owned(),
    };
    let candidate = Arc::new(ryo_benchmark::outbound::linter::factory::ryo_context(
        worker,
        staged_request.rules,
        staged_request.query_modules,
        staged_request.rule_applicability,
        staged_request.parser_configuration,
        staged_request.source_language,
        staged_request.project_context_mode,
        configured.clone(),
        ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations::default(),
    ));
    let reference = Arc::new(commands::reference(ruff, tool)?);
    let result = CompareCorpusUseCaseImpl::new(
        reference,
        candidate,
        Arc::new(ryo_benchmark::outbound::execution::factory::process()),
        Arc::new(ryo_benchmark::outbound::context::factory::local()),
        Arc::clone(suite_source),
        Arc::new(ryo_benchmark::outbound::evidence::factory::bmf(
            Box::new(std::io::stdout()),
            suite_source.create_text(ArtifactPath(&per_rule))?,
        )),
    )
    .compare_corpus(
        &CorpusParity {
            request: &request,
            configured: &configured,
            accepted: &accepted,
            reference: ContestantName(constants::REFERENCE_NAME),
        },
        observations,
    );
    drop(scratch);
    result
}
