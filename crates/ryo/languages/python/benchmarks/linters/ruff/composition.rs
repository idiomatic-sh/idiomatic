//! Python composition provides its linked language only at this outer edge.
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

pub(crate) fn run() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    let suite_source = ryo_benchmark::outbound::suite::factory::local();
    let execution = ryo_benchmark::outbound::execution::factory::process();
    let workspaces = ryo_benchmark::outbound::workspace::factory::git();
    let command = args.next();
    match command.as_deref() {
        Some("run-ryo") => {
            let input = args.next().ok_or_else(|| Error::Invalid("missing worker request".into()))?;
            let engine = ryo_benchmark::outbound::engine::factory::local(ryo::linter::Languages(vec![ryo_python::linked()]));
            let transport = ryo_benchmark::outbound::worker::factory::json(input.into(), Box::new(std::io::stdout()));
            RunRyoUseCaseImpl::new(Arc::new(engine), Arc::new(transport)).run_ryo()
        }
        Some("verify-cases") => {
            let executable = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected Ruff executable".into()))?).canonicalize()?;
            let mut tool = suite_source.profile(&directory())?;
            suite_source.verify_tool(&executable, &tool)?;
            let audit = suite_source.audit(&directory(), &tool)?;
            tool.defaults = audit.authored().to_vec();
            let reference = reference(executable, tool)?;
            let scratch = tempfile::tempdir()?;
            let count = ryo_benchmark::outbound::suite::adapters::local::adapter::verify_cases(&reference, &execution, &std::env::current_exe()?, &directory().join("rules"), scratch.path())?;
            println!("{count} cases passed against both contestants");
            Ok(())
        }
        Some("measure" | "measure-scoped" | "measure-scoped-valid") => {
            if cfg!(debug_assertions) { return Err(Error::Invalid("measurement requires a release-profile benchmark executable".into())); }
            let executable = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected Ruff executable".into()))?).canonicalize()?;
            let cache = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected owned cache directory".into()))?);
            std::fs::create_dir_all(&cache)?;
            let cache = cache.canonicalize()?;
            let mut tool = suite_source.profile(&directory())?;
            suite_source.verify_tool(&executable, &tool)?;
            let mut audit = suite_source.audit(&directory(), &tool)?;
            let authored_total = audit.authored_count();
            if let Some(selected) = args.next() {
                audit = audit.select(&selected.split(',').map(str::to_owned).collect::<Vec<_>>())?;
            }
            let scope = if command.as_deref() == Some("measure") { CoverageScope::Accounted } else { CoverageScope::Interim };
            if scope == CoverageScope::Accounted { require_accounted(&audit, &tool)?; }
            let default_count = tool.defaults.len();
            let deferred_count = tool.deferred.len();
            tool.defaults = audit.authored().to_vec();
            let reference = reference(executable.clone(), tool)?;
            let selected_scratch = tempfile::Builder::new().prefix("certification-").tempdir_in(&cache)?.keep();
            let selected_profile = selected_scratch.join("profile");
            suite_source.stage(&directory(), &selected_profile, &audit)?;
            let selected_rules = selected_profile.join("rules");
            let candidate_executable = selected_scratch.join("candidate");
            std::fs::copy(std::env::current_exe()?, &candidate_executable)?;
            let candidate = ryo_benchmark::outbound::linter::factory::ryo(candidate_executable.clone(), selected_rules.clone());
            let case_scratch = tempfile::tempdir()?;
            let certificate = ryo_benchmark::application::certify::service::certify(&suite_source, &reference, &execution, &selected_profile, &executable, &candidate_executable, &selected_rules, case_scratch.path(), scope)?;
            eprintln!("scope: {scope:?}; {} verified / {default_count} shipped defaults; {deferred_count} assessed deferrals; {} unassessed; {} authored outside subset; {} cases; certificate={}; artifacts={}", audit.authored_count(), default_count - authored_total - deferred_count, authored_total - audit.authored_count(), certificate.cases(), certificate.fingerprint(), selected_profile.display());
            eprintln!("environment: os={} arch={} release=true workers=1; GNU time CPU=user+system, RSS=peak KiB converted to bytes; wall includes collector startup; normalization and certificate/source hashing excluded; OS page cache uncontrolled; retained cache hits unverified", std::env::consts::OS, std::env::consts::ARCH);
            let suite: Suite = suite_source.projects(&directory().join("../.."))?;
            if suite.language != "python" || !(2..=3).contains(&suite.projects.len()) { return Err(Error::Invalid("Python suite requires 2–3 projects".into())); }
            let execution = ryo_benchmark::outbound::execution::factory::timed(suite.instrumentation.executable, suite.instrumentation.sha256);
            let evidence = ryo_benchmark::outbound::evidence::factory::bmf(Box::new(std::io::stdout()));
            let mut samples = Vec::new();
            let mut failures = Vec::new();
            for project in suite.projects {
                let observed = (|| -> Result<Vec<Sample>, Error> {
                let mut samples = Vec::new();
                let mut workspace = workspaces.prepare(&project, &cache.join("projects"))?;
                let discovered: Vec<String> = reference.files(workspace.root())?.into_iter().map(|path| path.0.into_string()).collect();
                let (mut files, excluded): (Vec<_>, Vec<_>) = discovered.into_iter().partition(|file| file.ends_with(".py") || file.ends_with(".pyi"));
                eprintln!("{}: matched Python source scope {} files; excluded default-discovered formats {} files (ryo-7tkn.2): {:?}", project.name, files.len(), excluded.len(), excluded);
                let tracked = workspaces.tracked_files(&workspace)?;
                if files.iter().any(|file| !tracked.contains(file)) { return Err(Error::Invalid("Ruff discovered untracked source".into())); }
                if command.as_deref() == Some("measure-scoped-valid") {
                    let preflight = RunRequest { root: workspace.root().to_owned(), files: files.clone(), cache: cache.join("preflight").join(&project.name) };
                    let parsed = execute(&reference, &execution, &preflight)?;
                    let invalid: std::collections::BTreeSet<_> = parsed.report.diagnostics.iter().filter(|finding| finding.rule == "invalid-syntax").map(|finding| finding.file.clone()).collect();
                    for finding in parsed.report.diagnostics.iter().filter(|finding| finding.rule == "invalid-syntax") {
                        eprintln!("{}: excluded invalid Python (pinned Ruff parser), path={} range={}..{}; frontend parity gap ryo-7tkn.2", project.name, finding.file, finding.start, finding.end);
                    }
                    files.retain(|file| !invalid.contains(file));
                }
                let paths: Vec<_> = files.iter().map(|file| workspace.root().join(file)).collect();
                let bytes = paths.iter().map(|path| std::fs::metadata(path).map(|metadata| metadata.len())).collect::<Result<Vec<_>, _>>()?.into_iter().sum::<u64>();
                eprintln!("{}: measured source files={} bytes={bytes} digest={} input-profile={}", project.name, files.len(), suite_source.fingerprint(&paths)?, if command.as_deref() == Some("measure-scoped-valid") { "pinned-reference-valid-python" } else { "all-python-sources" });
                for order in [RunOrder::ReferenceFirst, RunOrder::CandidateFirst].into_iter().cycle().take(6) {
                    // A new owned directory makes both retained and clean baselines fresh,
                    // even if an earlier trial left caches behind in the caller's root.
                    let trial = tempfile::Builder::new().prefix("trial-").tempdir_in(&cache)?.keep();
                    let request = RunRequest { root: workspace.root().to_owned(), files: files.clone(), cache: trial.join("retained") };
                    let clean_request = RunRequest { cache: trial.join("clean-edited"), ..request.clone() };
                    let cold = measure_certified(&certificate, &suite_source, &reference, &candidate, &execution, &request, order)?;
                    let mut counts = std::collections::BTreeMap::new();
                    for diagnostic in &cold.reference.report.diagnostics { *counts.entry(&diagnostic.rule).or_insert(0usize) += 1; }
                    eprintln!("{}: exact matching findings per rule: {counts:?}", project.name);
                    let retained = measure_certified(&certificate, &suite_source, &reference, &candidate, &execution, &request, order)?;
                    compare(&cold.reference.report, &retained.reference.report).map_err(|difference| Error::Invalid(format!("reference retained-cache mismatch: {difference:?}")))?;
                    compare(&cold.candidate.report, &retained.candidate.report).map_err(|difference| Error::Invalid(format!("candidate retained-cache mismatch: {difference:?}")))?;
                    workspaces.apply_patch(&mut workspace, &directory().join("../..").join(&project.patch))?;
                    let edited = measure_certified(&certificate, &suite_source, &reference, &candidate, &execution, &request, order);
                    let clean_reference = execute(&reference, &execution, &clean_request);
                    let clean_candidate = execute(&candidate, &execution, &clean_request);
                    workspaces.reset(&mut workspace)?;
                    let edited = edited?;
                    compare(&edited.reference.report, &clean_reference?.report).map_err(|difference| Error::Invalid(format!("reference edited retained vs fresh mismatch: {difference:?}")))?;
                    compare(&edited.candidate.report, &clean_candidate?.report).map_err(|difference| Error::Invalid(format!("candidate edited retained vs fresh mismatch: {difference:?}")))?;
                    for (condition, pair) in [("empty-tool-cache", cold), ("retained-tool-cache", retained), ("edited-tool-cache", edited)] {
                        for (contestant, measurement) in [("ruff", pair.reference), ("ryo", pair.candidate)] {
                            eprintln!("sample: project={} contestant={contestant} condition={condition} order={order:?} wall={:?} resources={:?}", project.name, measurement.wall, measurement.resources);
                            samples.push(Sample { name: format!("{}/{contestant}/{condition}", project.name), measurement });
                        }
                    }
                }
                Ok(samples)
                })();
                match observed { Ok(observed) => samples.extend(observed), Err(error) => { eprintln!("{}: corpus comparison failed: {error}", project.name); failures.push(project.name); } }
            }
            if !samples.is_empty() { evidence.publish(&samples)?; }
            if failures.is_empty() { Ok(()) } else { Err(Error::Invalid(format!("corpus comparisons failed: {failures:?}"))) }
        }
        Some("audit") => {
            let tool = suite_source.profile(&directory())?;
            let result = suite_source.audit(&directory(), &tool)?;
            println!("{} {}: {} queries authored / {} shipped defaults (not a parity certificate)", tool.tool, tool.version, result.authored_count(), tool.defaults.len());
            require_accounted(&result, &tool)
        }
        Some("verify-tool") => {
            let executable = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected executable path".into()))?);
            suite_source.verify_tool(&executable, &suite_source.profile(&directory())?)
        }
        _ => Err(Error::Invalid("usage: ruff benchmark: audit | verify-tool <executable> | measure <ruff> <fresh-cache> | run-ryo <json>".into())),
    }
}

fn directory() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff")
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
