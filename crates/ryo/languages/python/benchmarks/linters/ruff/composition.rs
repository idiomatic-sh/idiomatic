//! Python composition provides its linked language only at this outer edge.
mod commands;
mod constants;
mod diagnostic_protocol;
mod output;
mod request_input;
mod targets;
use ryo_benchmark::application::run::service::run as execute;
use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
use ryo_benchmark::domain::measurements::types::{RunOrder, sample::value::BenchmarkSample};
use ryo_benchmark::domain::suite::service::require_accounted;
use ryo_benchmark::domain::suite::types::{BenchmarkCoverage, RunRequest, Suite};
use ryo_benchmark::inbound::use_cases::certify::{
    interface::CertifyUseCase as _, use_case::CertifyUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::compare::{
    interface::CompareUseCase as _, use_case::CompareUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::measure::{
    interface::MeasureUseCase as _, use_case::MeasureUseCaseImpl,
};
use ryo_benchmark::outbound::evidence::interface::EvidencePort as _;
use ryo_benchmark::outbound::suite::types::{ArtifactPath, ArtifactSet};
use ryo_benchmark::outbound::workspace::interface::WorkspacePort as _;
use std::{path::PathBuf, sync::Arc};
/// Dispatch benchmark commands and retain the complete certification-to-observation timeline.
#[expect(
    clippy::too_many_lines,
    reason = "this outer composition keeps certification, explicit source filtering and cold/retained/edited equivalence in one ordered timeline; reusable algorithms live in the shared harness"
)]
pub(crate) fn run(
    output: &mut dyn std::io::Write,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut args = std::env::args()
        .skip(1)
        .filter(|argument| argument != constants::CARGO_BENCH_ARGUMENT);
    let suite_source: Arc<dyn ryo_benchmark::outbound::suite::interface::SuitePort> =
        Arc::new(ryo_benchmark::outbound::suite::factory::local());
    let comparison = CompareUseCaseImpl::new();
    let command = args.next();
    match command.as_deref() {
        Some(constants::RUN_RYO | constants::PROFILE_RYO | constants::DIAGNOSTIC_RYO) => {
            commands::worker(
                args.map(request_input::BenchmarkArgument),
                diagnostic_protocol::tracing_for_command(command.as_deref().unwrap_or_default())
                    .ok_or_else(|| Error::Invalid("unsupported worker command".into()))?,
            )
        }
        Some(constants::VERIFY_CASES) => {
            let count = commands::verify_cases(args.map(request_input::BenchmarkArgument))?;
            writeln!(output, "{count} cases passed against both contestants")?;
            Ok(())
        }
        Some(constants::VERIFY_REFERENCE_CASES) => {
            let count =
                commands::verify_reference_cases(args.map(request_input::BenchmarkArgument))?;
            writeln!(
                output,
                "{count} cases matched the reference; no candidate was executed"
            )?;
            Ok(())
        }
        Some(
            constants::MEASURE
            | constants::MEASURE_ACCOUNTED_VALID
            | constants::MEASURE_SCOPED
            | constants::MEASURE_VALID,
        ) => {
            if cfg!(debug_assertions) {
                return Err(Error::Invalid(constants::RELEASE_REQUIRED.into()));
            }
            let executable = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
            )
            .canonicalize()?;
            let cache = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(constants::EXPECTED_CACHE.into()))?,
            );
            let collector_path = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(constants::EXPECTED_COLLECTOR.into()))?,
            )
            .canonicalize()?;
            std::fs::create_dir_all(&cache)?;
            let cache = cache.canonicalize()?;
            let mut tool = suite_source.profile(ArtifactPath(&commands::directory()))?;
            suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
            let mut audit = suite_source.audit(ArtifactPath(&commands::directory()), &tool)?;
            let authored_total = audit.authored_count();
            if let Some(selected) = args.next() {
                audit = audit.select(
                    &selected
                        .split(',')
                        .map(ryo::linter::RuleId::try_from)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                )?;
            }
            let scope = if matches!(
                command.as_deref(),
                Some(constants::MEASURE | constants::MEASURE_ACCOUNTED_VALID)
            ) {
                BenchmarkCoverage::Accounted
            } else {
                BenchmarkCoverage::Interim
            };
            if scope == BenchmarkCoverage::Accounted {
                require_accounted(&audit, &tool)?;
            }
            let default_count = tool.defaults.len();
            let deferred_count = tool.deferred.len();
            let pending_count = tool.pending.len();
            let query_modules = tool.query_modules.clone();
            let rule_applicability = tool.rule_applicability.clone();
            let parser_configuration = tool.parser_configuration.clone();
            let source_language = tool.source_language.clone();
            let project_context_mode = tool.project_context_mode;
            tool.defaults = audit.authored().to_vec();
            let reference = Arc::new(commands::reference(executable.clone(), tool)?);
            let selected_scratch = tempfile::Builder::new()
                .prefix(constants::CERTIFICATION_PREFIX)
                .tempdir_in(&cache)?
                .keep();
            let selected_profile = selected_scratch.join(constants::PROFILE_DIRECTORY);
            suite_source.stage(
                ArtifactPath(&commands::directory()),
                ArtifactPath(&selected_profile),
                &audit,
            )?;
            let frozen_workload = selected_scratch.join(constants::WORKLOAD_DIRECTORY);
            suite_source.stage_projects(
                ArtifactPath(&commands::directory().join(constants::LANGUAGE_DIRECTORY)),
                ArtifactPath(&frozen_workload),
            )?;
            let collector_settings =
                commands::collector(suite_source.as_ref(), &collector_path, &frozen_workload)?;
            let execution = commands::timed(collector_settings, observations)?;
            let selected_rules = selected_profile.join(constants::RULES_DIRECTORY);
            let candidate_executable = selected_scratch.join(constants::CANDIDATE_EXECUTABLE);
            let _bytes = std::fs::copy(std::env::current_exe()?, &candidate_executable)?;
            let candidate: Arc<dyn ryo_benchmark::outbound::linter::interface::LinterPort> =
                Arc::new(ryo_benchmark::outbound::linter::factory::ryo_context(
                    candidate_executable.clone(),
                    selected_rules.clone(),
                    query_modules,
                    rule_applicability,
                    parser_configuration,
                    source_language,
                    project_context_mode,
                ));
            let case_scratch = tempfile::tempdir()?;
            let certificate = CertifyUseCaseImpl::new(
                Arc::<_>::clone(&suite_source),
                Arc::<super::adapter::LinterRuffAdapter>::clone(&reference),
                Arc::<_>::clone(&candidate),
                Arc::<_>::clone(&execution),
            )
            .certify(
                &selected_profile,
                &frozen_workload,
                &executable,
                &candidate_executable,
                &selected_rules,
                case_scratch.path(),
                scope,
            )?;
            writeln!(
                observations,
                "scope: {}; {} verified / {default_count} shipped defaults; {deferred_count} assessed deferrals; {pending_count} assessed pending; {} unassessed; {} authored outside subset; {} cases; certificate={}; artifacts={}",
                match scope {
                    BenchmarkCoverage::Accounted => constants::ACCOUNTED,
                    BenchmarkCoverage::Interim => constants::INTERIM,
                },
                audit.authored_count(),
                default_count - authored_total.0 - deferred_count - pending_count,
                authored_total.0 - audit.authored_count().0,
                certificate.cases(),
                certificate.fingerprint(),
                selected_profile.display()
            )?;
            output::environment(observations)?;
            let suite: Suite = suite_source.projects(ArtifactPath(&frozen_workload))?;
            if suite.language.0 != constants::PYTHON || !(2..=3).contains(&suite.projects.len()) {
                return Err(Error::Invalid(constants::PROJECT_COUNT_ERROR.into()));
            }
            let measurements = MeasureUseCaseImpl::new(
                Arc::<super::adapter::LinterRuffAdapter>::clone(&reference),
                Arc::<_>::clone(&candidate),
                Arc::<_>::clone(&execution),
                Arc::<_>::clone(&suite_source),
            );
            let evidence =
                ryo_benchmark::outbound::evidence::factory::bmf(Box::new(std::io::stdout()));
            let mut samples = Vec::new();
            let mut failures = Vec::new();
            for project in suite.projects {
                let observed = (|| -> Result<Vec<BenchmarkSample>, Error> {
                    let mut project_samples = Vec::new();
                    let workspaces = ryo_benchmark::outbound::workspace::factory::git(
                        cache.join(constants::PROJECTS_DIRECTORY),
                    );
                    let mut workspace = workspaces.prepare(&project)?;
                    let discovered: Vec<ryo::linter::RelativePath> =
                        reference.files(workspace.root())?.into_iter().collect();
                    let (mut files, excluded): (Vec<_>, Vec<_>) =
                        discovered.into_iter().partition(|file| {
                            file.0.ends_with(constants::PYTHON_EXTENSION)
                                || file.0.ends_with(constants::STUB_EXTENSION)
                        });
                    writeln!(
                        observations,
                        "{}: matched Python source scope {} files; excluded default-discovered formats {} files (source-kind capability gap): {}",
                        project.name,
                        files.len(),
                        excluded.len(),
                        excluded
                            .iter()
                            .map(|path| path.0.as_ref())
                            .collect::<Vec<_>>()
                            .join(constants::PATH_SEPARATOR)
                    )?;
                    let tracked = workspaces.tracked_files(&workspace)?.0;
                    if files.iter().any(|file| {
                        !tracked
                            .iter()
                            .any(|tracked| tracked.as_str() == file.0.as_ref())
                    }) {
                        return Err(Error::Invalid(constants::UNTRACKED_SOURCE.into()));
                    }
                    if matches!(
                        command.as_deref(),
                        Some(constants::MEASURE_VALID | constants::MEASURE_ACCOUNTED_VALID)
                    ) {
                        let preflight = RunRequest {
                            file_metadata: None,
                            context_manifest: None,
                            root: workspace.root().to_owned(),
                            files: files.clone(),
                            state: cache
                                .join(constants::PREFLIGHT_DIRECTORY)
                                .join(&project.name.0),
                        };
                        let parsed = execute(reference.as_ref(), execution.as_ref(), &preflight)?;
                        let invalid: std::collections::BTreeSet<_> = parsed
                            .report
                            .diagnostics
                            .iter()
                            .filter(|finding| finding.rule.as_ref() == constants::INVALID_SYNTAX)
                            .map(|finding| finding.file.clone())
                            .collect();
                        for finding in
                            parsed.report.diagnostics.iter().filter(|finding| {
                                finding.rule.as_ref() == constants::INVALID_SYNTAX
                            })
                        {
                            writeln!(
                                observations,
                                "{}: excluded invalid Python (pinned Ruff parser), path={} range={}..{}; frontend parser capability gap",
                                project.name, finding.file.0, finding.start.0, finding.end.0
                            )?;
                        }
                        files.retain(|file| !invalid.contains(file));
                    }
                    let paths: Vec<_> = files
                        .iter()
                        .map(|file| workspace.root().join(file.0.as_ref()))
                        .collect();
                    let bytes = paths
                        .iter()
                        .map(|path| std::fs::metadata(path).map(|metadata| metadata.len()))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .sum::<u64>();
                    writeln!(
                        observations,
                        "{}: measured source files={} bytes={bytes} digest={} input-profile={}",
                        project.name,
                        files.len(),
                        suite_source.fingerprint(ArtifactSet(&paths))?,
                        if matches!(
                            command.as_deref(),
                            Some(constants::MEASURE_VALID | constants::MEASURE_ACCOUNTED_VALID)
                        ) {
                            constants::VALID_SOURCE_PROFILE
                        } else {
                            constants::FULL_SOURCE_PROFILE
                        }
                    )?;
                    for order in [RunOrder::ReferenceFirst, RunOrder::CandidateFirst]
                        .into_iter()
                        .cycle()
                        .take(6)
                    {
                        // A new owned directory makes both retained and clean baselines fresh,
                        // even if an earlier trial left caches behind in the caller's root.
                        let trial = tempfile::Builder::new()
                            .prefix(constants::TRIAL_PREFIX)
                            .tempdir_in(&cache)?
                            .keep();
                        let request = RunRequest {
                            file_metadata: None,
                            context_manifest: None,
                            root: workspace.root().to_owned(),
                            files: files.clone(),
                            state: trial.join(constants::RETAINED_DIRECTORY),
                        };
                        let clean_request = RunRequest {
                            state: trial.join(constants::CLEAN_EDITED_DIRECTORY),
                            ..request.clone()
                        };
                        let cold = measurements.measure(&certificate, &request, order)?;
                        let counts = output::diagnostic_counts(&cold.reference().report);
                        writeln!(
                            observations,
                            "{}: exact matching findings per rule: {counts}",
                            project.name
                        )?;
                        let retained = measurements.measure(&certificate, &request, order)?;
                        comparison
                            .compare(&cold.reference().report, &retained.reference().report)
                            .map_err(|difference| {
                                Error::Invalid(format!(
                                    "reference retained-cache mismatch: {difference:?}"
                                ))
                            })?;
                        comparison
                            .compare(&cold.candidate().report, &retained.candidate().report)
                            .map_err(|difference| {
                                Error::Invalid(format!(
                                    "candidate retained-cache mismatch: {difference:?}"
                                ))
                            })?;
                        workspaces
                            .apply_patch(&mut workspace, &frozen_workload.join(&project.patch))?;
                        let edited = measurements.measure(&certificate, &request, order);
                        let clean_reference =
                            execute(reference.as_ref(), execution.as_ref(), &clean_request);
                        let clean_candidate =
                            execute(candidate.as_ref(), execution.as_ref(), &clean_request);
                        workspaces.reset(&mut workspace)?;
                        let edited = edited?;
                        comparison
                            .compare(&edited.reference().report, &clean_reference?.report)
                            .map_err(|difference| {
                                Error::Invalid(format!(
                                    "reference edited retained vs fresh mismatch: {difference:?}"
                                ))
                            })?;
                        comparison
                            .compare(&edited.candidate().report, &clean_candidate?.report)
                            .map_err(|difference| {
                                Error::Invalid(format!(
                                    "candidate edited retained vs fresh mismatch: {difference:?}"
                                ))
                            })?;
                        for (condition, pair) in [
                            (constants::EMPTY_CACHE_CONDITION, cold),
                            (constants::RETAINED_CACHE_CONDITION, retained),
                            (constants::EDITED_CACHE_CONDITION, edited),
                        ] {
                            let contestants =
                                [constants::REFERENCE_NAME, constants::CANDIDATE_NAME];
                            let named = pair.samples(contestants.map(|contestant| {
                                ryo_benchmark::domain::measurements::types::SampleName(format!(
                                    "{}/{contestant}/{condition}",
                                    project.name
                                ))
                            }));
                            for (contestant, sample) in contestants.into_iter().zip(named) {
                                let observed_counts =
                                    output::diagnostic_counts(&sample.measurement().report);
                                writeln!(
                                    observations,
                                    "findings: project={} contestant={contestant} condition={condition} counts={observed_counts}",
                                    project.name
                                )?;
                                output::observe_sample(observations, &sample, order)?;
                                project_samples.push(sample);
                            }
                        }
                    }
                    Ok(project_samples)
                })();
                match observed {
                    Ok(observed) => samples.extend(observed),
                    Err(error) => {
                        writeln!(
                            observations,
                            "{}: corpus comparison failed: {error}",
                            project.name
                        )?;
                        failures.push(project.name);
                    }
                }
            }
            // The evidence boundary refuses an empty observation set; never bypass that guard.
            evidence.publish(&samples)?;
            if failures.is_empty() {
                Ok(())
            } else {
                Err(Error::Invalid(format!(
                    "corpus comparisons failed: {failures:?}"
                )))
            }
        }
        Some(constants::AUDIT) => commands::audit(suite_source.as_ref(), output),
        Some(constants::VERIFY_TOOL) => {
            let executable = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(constants::EXPECTED_EXECUTABLE.into()))?,
            );
            suite_source.verify_tool(
                ArtifactPath(&executable),
                &suite_source.profile(ArtifactPath(&commands::directory()))?,
            )
        }
        _ => Err(Error::Invalid(constants::USAGE.into())),
    }
}
