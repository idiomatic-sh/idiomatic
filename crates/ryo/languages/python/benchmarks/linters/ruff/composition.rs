//! Python composition provides its linked language only at this outer edge.
mod constants;
use super::{adapter::LinterRuffAdapter, settings::LinterRuffSettings};
use ryo_benchmark::application::run::service::run as execute;
use ryo_benchmark::domain::diagnostics::errors::Error;
use ryo_benchmark::domain::measurements::types::{RunOrder, Sample};
use ryo_benchmark::domain::suite::service::require_accounted;
use ryo_benchmark::domain::suite::types::{CoverageScope, RunRequest, Suite, Tool};
use ryo_benchmark::inbound::use_cases::compare::service::compare;
use ryo_benchmark::inbound::use_cases::measure::service::measure_certified;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::evidence::interface::EvidencePort;
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark::outbound::workspace::interface::WorkspacePort;
use std::{path::PathBuf, sync::Arc};

pub(crate) fn run(
    output: &mut dyn std::io::Write,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut args = std::env::args()
        .skip(1)
        .filter(|argument| argument != constants::CARGO_BENCH_ARGUMENT);
    let suite_source = ryo_benchmark::outbound::suite::factory::local();
    let execution = ryo_benchmark::outbound::execution::factory::process();
    let command = args.next();
    match command.as_deref() {
        Some(constants::RUN_RYO | constants::PROFILE_RYO) => {
            let input = args
                .next()
                .ok_or_else(|| Error::Invalid(constants::MISSING_WORKER_REQUEST.into()))?;
            let languages = ryo::linter::Languages(vec![ryo_python::linked()]);
            let engine: Arc<dyn ryo_benchmark::outbound::engine::interface::EnginePort> =
                match if command.as_deref() == Some(constants::PROFILE_RYO) {
                    Some(
                        args.next()
                            .ok_or_else(|| Error::Invalid(constants::MISSING_TRACE.into()))?,
                    )
                } else {
                    None
                } {
                    Some(path) => Arc::new(ryo_benchmark::outbound::engine::factory::profiled(
                        languages,
                        path.into(),
                    )),
                    None => Arc::new(ryo_benchmark::outbound::engine::factory::local(languages)),
                };
            let transport = ryo_benchmark::outbound::worker::factory::json(
                input.into(),
                Box::new(std::io::stdout()),
            );
            RunRyoUseCaseImpl::new(engine, Arc::new(transport)).run_ryo()
        }
        Some(constants::VERIFY_CASES) => {
            let executable = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
            )
            .canonicalize()?;
            let mut tool = suite_source.profile(&directory())?;
            suite_source.verify_tool(&executable, &tool)?;
            let audit = suite_source.audit(&directory(), &tool)?;
            tool.defaults = audit.authored().to_vec();
            let reference = reference(executable, tool)?;
            let scratch = tempfile::tempdir()?;
            let profile = scratch.path().join(constants::PROFILE_DIRECTORY);
            suite_source.stage(&directory(), &profile, &audit)?;
            let candidate = ryo_benchmark::outbound::linter::factory::ryo(
                std::env::current_exe()?,
                profile.join(constants::RULES_DIRECTORY),
            );
            let count = ryo_benchmark::application::cases::service::verify(
                &suite_source,
                &reference,
                &candidate,
                &execution,
                &profile.join(constants::RULES_DIRECTORY),
                &scratch.path().join(constants::CASES_DIRECTORY),
            )?;
            writeln!(output, "{count} cases passed against both contestants")?;
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
            std::fs::create_dir_all(&cache)?;
            let cache = cache.canonicalize()?;
            let mut tool = suite_source.profile(&directory())?;
            suite_source.verify_tool(&executable, &tool)?;
            let mut audit = suite_source.audit(&directory(), &tool)?;
            let authored_total = audit.authored_count();
            if let Some(selected) = args.next() {
                audit =
                    audit.select(&selected.split(',').map(str::to_owned).collect::<Vec<_>>())?;
            }
            let scope = if matches!(
                command.as_deref(),
                Some(constants::MEASURE | constants::MEASURE_ACCOUNTED_VALID)
            ) {
                CoverageScope::Accounted
            } else {
                CoverageScope::Interim
            };
            if scope == CoverageScope::Accounted {
                require_accounted(&audit, &tool)?;
            }
            let default_count = tool.defaults.len();
            let deferred_count = tool.deferred.len();
            let pending_count = tool.pending.len();
            tool.defaults = audit.authored().to_vec();
            let reference = reference(executable.clone(), tool)?;
            let selected_scratch = tempfile::Builder::new()
                .prefix(constants::CERTIFICATION_PREFIX)
                .tempdir_in(&cache)?
                .keep();
            let selected_profile = selected_scratch.join(constants::PROFILE_DIRECTORY);
            suite_source.stage(&directory(), &selected_profile, &audit)?;
            let frozen_workload = selected_scratch.join(constants::WORKLOAD_DIRECTORY);
            suite_source.stage_projects(
                &directory().join(constants::LANGUAGE_DIRECTORY),
                &frozen_workload,
            )?;
            let selected_rules = selected_profile.join(constants::RULES_DIRECTORY);
            let candidate_executable = selected_scratch.join(constants::CANDIDATE_EXECUTABLE);
            let _bytes = std::fs::copy(std::env::current_exe()?, &candidate_executable)?;
            let candidate = ryo_benchmark::outbound::linter::factory::ryo(
                candidate_executable.clone(),
                selected_rules.clone(),
            );
            let case_scratch = tempfile::tempdir()?;
            let certificate = ryo_benchmark::application::certify::service::certify(
                &suite_source,
                &reference,
                &candidate,
                &execution,
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
                "scope: {scope:?}; {} verified / {default_count} shipped defaults; {deferred_count} assessed deferrals; {pending_count} assessed pending; {} unassessed; {} authored outside subset; {} cases; certificate={}; artifacts={}",
                audit.authored_count(),
                default_count - authored_total - deferred_count - pending_count,
                authored_total - audit.authored_count(),
                certificate.cases(),
                certificate.fingerprint(),
                selected_profile.display()
            )?;
            writeln!(
                observations,
                "environment: os={} arch={} release=true workers=1; Linux wait4 CPU=user+system, RSS=peak bytes; wall includes child spawn/wait and pipe collection; normalization and certificate/source hashing excluded; OS page cache uncontrolled; retained cache hits unverified",
                std::env::consts::OS,
                std::env::consts::ARCH
            )?;
            let suite: Suite = suite_source.projects(&frozen_workload)?;
            if suite.language != constants::PYTHON || !(2..=3).contains(&suite.projects.len()) {
                return Err(Error::Invalid(constants::PROJECT_COUNT_ERROR.into()));
            }
            let execution = ryo_benchmark::outbound::execution::factory::timed();
            let evidence =
                ryo_benchmark::outbound::evidence::factory::bmf(Box::new(std::io::stdout()));
            let mut samples = Vec::new();
            let mut failures = Vec::new();
            for project in suite.projects {
                let observed = (|| -> Result<Vec<Sample>, Error> {
                    let mut samples = Vec::new();
                    let workspaces = ryo_benchmark::outbound::workspace::factory::git(
                        cache.join(constants::PROJECTS_DIRECTORY),
                    );
                    let mut workspace = workspaces.prepare(&project)?;
                    let discovered: Vec<String> = reference
                        .files(workspace.root())?
                        .into_iter()
                        .map(|path| path.0.into_string())
                        .collect();
                    let (mut files, excluded): (Vec<_>, Vec<_>) =
                        discovered.into_iter().partition(|file| {
                            file.ends_with(constants::PYTHON_EXTENSION)
                                || file.ends_with(constants::STUB_EXTENSION)
                        });
                    writeln!(
                        observations,
                        "{}: matched Python source scope {} files; excluded default-discovered formats {} files (source-kind capability gap): {:?}",
                        project.name,
                        files.len(),
                        excluded.len(),
                        excluded
                    )?;
                    let tracked = workspaces.tracked_files(&workspace)?.0;
                    if files.iter().any(|file| !tracked.contains(file)) {
                        return Err(Error::Invalid(constants::UNTRACKED_SOURCE.into()));
                    }
                    if matches!(
                        command.as_deref(),
                        Some(constants::MEASURE_VALID | constants::MEASURE_ACCOUNTED_VALID)
                    ) {
                        let preflight = RunRequest {
                            root: workspace.root().to_owned(),
                            files: files.clone(),
                            cache: cache
                                .join(constants::PREFLIGHT_DIRECTORY)
                                .join(&project.name),
                        };
                        let parsed = execute(&reference, &execution, &preflight)?;
                        let invalid: std::collections::BTreeSet<_> = parsed
                            .report
                            .diagnostics
                            .iter()
                            .filter(|finding| finding.rule == constants::INVALID_SYNTAX)
                            .map(|finding| finding.file.clone())
                            .collect();
                        for finding in parsed
                            .report
                            .diagnostics
                            .iter()
                            .filter(|finding| finding.rule == constants::INVALID_SYNTAX)
                        {
                            writeln!(
                                observations,
                                "{}: excluded invalid Python (pinned Ruff parser), path={} range={}..{}; frontend parser capability gap",
                                project.name, finding.file, finding.start, finding.end
                            )?;
                        }
                        files.retain(|file| !invalid.contains(file));
                    }
                    let paths: Vec<_> = files
                        .iter()
                        .map(|file| workspace.root().join(file))
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
                        suite_source.fingerprint(&paths)?,
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
                            root: workspace.root().to_owned(),
                            files: files.clone(),
                            cache: trial.join(constants::RETAINED_DIRECTORY),
                        };
                        let clean_request = RunRequest {
                            cache: trial.join(constants::CLEAN_EDITED_DIRECTORY),
                            ..request.clone()
                        };
                        let cold = measure_certified(
                            &certificate,
                            &suite_source,
                            &reference,
                            &candidate,
                            &execution,
                            &request,
                            order,
                        )?;
                        let counts = diagnostic_counts(&cold.reference.report);
                        writeln!(
                            observations,
                            "{}: exact matching findings per rule: {counts:?}",
                            project.name
                        )?;
                        let retained = measure_certified(
                            &certificate,
                            &suite_source,
                            &reference,
                            &candidate,
                            &execution,
                            &request,
                            order,
                        )?;
                        compare(&cold.reference.report, &retained.reference.report).map_err(
                            |difference| {
                                Error::Invalid(format!(
                                    "reference retained-cache mismatch: {difference:?}"
                                ))
                            },
                        )?;
                        compare(&cold.candidate.report, &retained.candidate.report).map_err(
                            |difference| {
                                Error::Invalid(format!(
                                    "candidate retained-cache mismatch: {difference:?}"
                                ))
                            },
                        )?;
                        workspaces
                            .apply_patch(&mut workspace, &frozen_workload.join(&project.patch))?;
                        let edited = measure_certified(
                            &certificate,
                            &suite_source,
                            &reference,
                            &candidate,
                            &execution,
                            &request,
                            order,
                        );
                        let clean_reference = execute(&reference, &execution, &clean_request);
                        let clean_candidate = execute(&candidate, &execution, &clean_request);
                        workspaces.reset(&mut workspace)?;
                        let edited = edited?;
                        compare(&edited.reference.report, &clean_reference?.report).map_err(
                            |difference| {
                                Error::Invalid(format!(
                                    "reference edited retained vs fresh mismatch: {difference:?}"
                                ))
                            },
                        )?;
                        compare(&edited.candidate.report, &clean_candidate?.report).map_err(
                            |difference| {
                                Error::Invalid(format!(
                                    "candidate edited retained vs fresh mismatch: {difference:?}"
                                ))
                            },
                        )?;
                        for (condition, pair) in [
                            (constants::EMPTY_CACHE_CONDITION, cold),
                            (constants::RETAINED_CACHE_CONDITION, retained),
                            (constants::EDITED_CACHE_CONDITION, edited),
                        ] {
                            for (contestant, measurement) in [
                                (constants::REFERENCE_NAME, pair.reference),
                                (constants::CANDIDATE_NAME, pair.candidate),
                            ] {
                                let counts = diagnostic_counts(&measurement.report);
                                writeln!(
                                    observations,
                                    "findings: project={} contestant={contestant} condition={condition} counts={counts:?}",
                                    project.name
                                )?;
                                writeln!(
                                    observations,
                                    "sample: project={} contestant={contestant} condition={condition} order={order:?} wall={:?} resources={:?}",
                                    project.name, measurement.wall, measurement.resources
                                )?;
                                samples.push(Sample {
                                    name: format!("{}/{contestant}/{condition}", project.name),
                                    measurement,
                                });
                            }
                        }
                    }
                    Ok(samples)
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
            if !samples.is_empty() {
                evidence.publish(&samples)?;
            }
            if failures.is_empty() {
                Ok(())
            } else {
                Err(Error::Invalid(format!(
                    "corpus comparisons failed: {failures:?}"
                )))
            }
        }
        Some(constants::AUDIT) => {
            let tool = suite_source.profile(&directory())?;
            let result = suite_source.audit(&directory(), &tool)?;
            writeln!(
                output,
                "{} {}: {} queries authored / {} shipped defaults (not a parity certificate)",
                tool.tool,
                tool.version,
                result.authored_count(),
                tool.defaults.len()
            )?;
            require_accounted(&result, &tool)
        }
        Some(constants::VERIFY_TOOL) => {
            let executable = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(constants::EXPECTED_EXECUTABLE.into()))?,
            );
            suite_source.verify_tool(&executable, &suite_source.profile(&directory())?)
        }
        _ => Err(Error::Invalid(constants::USAGE.into())),
    }
}

fn directory() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(constants::RUFF_DIRECTORY)
}

fn reference(executable: PathBuf, tool: Tool) -> Result<LinterRuffAdapter, Error> {
    let defaults = tool
        .defaults
        .into_iter()
        .map(|code| {
            ryo::linter::RuleId::try_from(code.as_str())
                .map_err(|error| Error::Invalid(error.to_string()))
        })
        .collect::<Result<_, _>>()?;
    Ok(LinterRuffAdapter::new(LinterRuffSettings {
        executable,
        defaults,
    }))
}

fn diagnostic_counts(
    report: &ryo_benchmark::domain::diagnostics::types::RunReport,
) -> std::collections::BTreeMap<&str, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for diagnostic in &report.diagnostics {
        *counts.entry(diagnostic.rule.as_str()).or_default() += constants::ONE_FINDING;
    }
    counts
}
