//! Python composition provides its linked language only at this outer edge.
use super::{adapter::Ruff, settings};
use ryo_benchmark::application::run::service::run as execute;
use ryo_benchmark::domain::diagnostics::errors::Error;
use ryo_benchmark::domain::measurements::types::RunOrder;
use ryo_benchmark::domain::suite::service::require_complete;
use ryo_benchmark::domain::suite::types::{RunRequest, Suite};
use ryo_benchmark::inbound::use_cases::compare::service::compare;
use ryo_benchmark::inbound::use_cases::measure::service::measure;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::suite::adapters::local::adapter::{audit, verify_executable};
use ryo_benchmark::outbound::workspace::interface::WorkspacePort;
use std::{path::PathBuf, sync::Arc};

pub(crate) fn run() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    let execution = ryo_benchmark::outbound::execution::factory::process();
    let workspaces = ryo_benchmark::outbound::workspace::factory::git();
    match args.next().as_deref() {
        Some("run-ryo") => {
            let input = args.next().ok_or_else(|| Error::Invalid("missing worker request".into()))?;
            let engine = ryo_benchmark::outbound::engine::factory::local(ryo::linter::Languages(vec![ryo_python::linked()]));
            let transport = ryo_benchmark::outbound::worker::factory::json(input.into(), Box::new(std::io::stdout()));
            RunRyoUseCaseImpl::new(Arc::new(engine), Arc::new(transport)).run_ryo()
        }
        Some("verify-cases") => {
            let executable = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected Ruff executable".into()))?).canonicalize()?;
            let tool = settings::load()?;
            verify_executable(&executable, &tool)?;
            let reference = Ruff { executable, defaults: tool.defaults.into_iter().collect() };
            let scratch = tempfile::tempdir()?;
            let count = ryo_benchmark::outbound::suite::adapters::local::adapter::verify_cases(&reference, &execution, &std::env::current_exe()?, &settings::directory().join("rules"), scratch.path())?;
            println!("{count} cases passed against both contestants");
            Ok(())
        }
        Some("measure") => {
            let executable = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected Ruff executable".into()))?).canonicalize()?;
            let cache = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected owned cache directory".into()))?);
            std::fs::create_dir_all(&cache)?;
            let cache = cache.canonicalize()?;
            let tool = settings::load()?;
            verify_executable(&executable, &tool)?;
            let audit = audit(&settings::directory(), &tool)?;
            require_complete(&audit)?;
            let reference = Ruff { executable, defaults: tool.defaults.into_iter().collect() };
            let candidate = ryo_benchmark::outbound::linter::factory::ryo(std::env::current_exe()?, settings::directory().join("rules"));
            let case_scratch = tempfile::tempdir()?;
            let _ = ryo_benchmark::outbound::suite::adapters::local::adapter::verify_cases(&reference, &execution, &std::env::current_exe()?, &settings::directory().join("rules"), case_scratch.path())?;
            let suite: Suite = ryo_benchmark::outbound::suite::adapters::local::schemas::suite(&std::fs::read_to_string(settings::directory().join("../../benchmark.toml"))?)?;
            if suite.language != "python" || !(2..=3).contains(&suite.projects.len()) { return Err(Error::Invalid("Python suite requires 2–3 projects".into())); }
            for project in suite.projects {
                let mut workspace = workspaces.prepare(&project, &cache.join("projects"))?;
                let files = reference.files(workspace.root())?;
                let tracked = workspaces.tracked_files(&workspace)?;
                if files.iter().any(|file| !tracked.contains(file)) { return Err(Error::Invalid("Ruff discovered untracked source".into())); }
                for order in [RunOrder::ReferenceFirst, RunOrder::CandidateFirst] {
                    // A new owned directory makes both retained and clean baselines fresh,
                    // even if an earlier trial left caches behind in the caller's root.
                    let trial = tempfile::Builder::new().prefix("trial-").tempdir_in(&cache)?.keep();
                    let request = RunRequest { root: workspace.root().to_owned(), files: files.clone(), cache: trial.join("retained") };
                    let clean_request = RunRequest { cache: trial.join("clean-edited"), ..request.clone() };
                    let cold = measure(&audit, &reference, &candidate, &execution, &request, order)?;
                    let retained = measure(&audit, &reference, &candidate, &execution, &request, order)?;
                    compare(&cold.reference.report, &retained.reference.report).map_err(|difference| Error::Invalid(format!("reference retained-cache mismatch: {difference:?}")))?;
                    compare(&cold.candidate.report, &retained.candidate.report).map_err(|difference| Error::Invalid(format!("candidate retained-cache mismatch: {difference:?}")))?;
                    workspaces.apply_patch(&mut workspace, &settings::directory().join("../..").join(&project.patch))?;
                    let edited = measure(&audit, &reference, &candidate, &execution, &request, order);
                    let clean_reference = execute(&reference, &execution, &clean_request);
                    let clean_candidate = execute(&candidate, &execution, &clean_request);
                    workspaces.reset(&mut workspace)?;
                    let edited = edited?;
                    compare(&edited.reference.report, &clean_reference?.report).map_err(|difference| Error::Invalid(format!("reference edited retained vs fresh mismatch: {difference:?}")))?;
                    compare(&edited.candidate.report, &clean_candidate?.report).map_err(|difference| Error::Invalid(format!("candidate edited retained vs fresh mismatch: {difference:?}")))?;
                    println!("{} {order:?} empty-tool-cache={:?}/{:?} retained-tool-cache={:?}/{:?} edited={:?}/{:?}; one worker each; OS page cache uncontrolled; cache hits unverified; trial={} (reference/ryo wall only)", project.name, cold.reference.wall, cold.candidate.wall, retained.reference.wall, retained.candidate.wall, edited.reference.wall, edited.candidate.wall, trial.display());
                }
            }
            Ok(())
        }
        Some("audit") => {
            let tool = settings::load()?;
            let result = audit(&settings::directory(), &tool)?;
            println!("{} {}: {} queries authored / {} shipped defaults (not a parity certificate)", tool.tool, tool.version, result.authored_count(), tool.defaults.len());
            require_complete(&result)
        }
        Some("verify-tool") => {
            let executable = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected executable path".into()))?);
            verify_executable(&executable, &settings::load()?)
        }
        _ => Err(Error::Invalid("usage: ruff benchmark: audit | verify-tool <executable> | measure <ruff> <fresh-cache> | run-ryo <json>".into())),
    }
}
