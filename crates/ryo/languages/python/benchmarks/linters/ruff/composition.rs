//! Python composition provides its linked language only at this outer edge.
use super::{adapter::Ruff, settings};
use ryo_benchmark::application::run::service::run as execute;
use ryo_benchmark::domain::diagnostics::errors::Error;
use ryo_benchmark::domain::suite::service::require_complete;
use ryo_benchmark::domain::suite::types::WorkerRequest;
use ryo_benchmark::domain::suite::types::{RunRequest, Suite};
use ryo_benchmark::inbound::use_cases::compare::service::compare;
use ryo_benchmark::inbound::use_cases::measure::service::measure;
use ryo_benchmark::outbound::{
    engine::adapters::local::adapter::run_ryo,
    suite::adapters::local::adapter::{audit, verify_executable},
};
use ryo_benchmark::outbound::{
    execution::adapters::process::adapter::ExecutionProcessAdapter,
    linter::adapters::ryo::adapter::LinterRyoAdapter,
    workspace::{adapters::git::adapter::WorkspaceGitAdapter, interface::WorkspacePort},
};
use std::path::PathBuf;

pub(crate) fn run() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("run-ryo") => {
            let input = args.next().ok_or_else(|| Error::Invalid("missing worker request".into()))?;
            let request: WorkerRequest = ryo_benchmark::outbound::linter::adapters::ryo::schemas::decode_request(&input)?;
            let report = run_ryo(ryo::linter::Languages(vec![ryo_python::linked()]), request)?;
            println!("{}", ryo_benchmark::outbound::linter::adapters::ryo::schemas::encode_report(report)?);
            Ok(())
        }
        Some("verify-cases") => {
            let executable = PathBuf::from(args.next().ok_or_else(|| Error::Invalid("expected Ruff executable".into()))?).canonicalize()?;
            let tool = settings::load()?;
            verify_executable(&executable, &tool)?;
            let reference = Ruff { executable, defaults: tool.defaults.into_iter().collect() };
            let scratch = tempfile::tempdir()?;
            let count = ryo_benchmark::outbound::suite::adapters::local::adapter::verify_cases(&reference, &ExecutionProcessAdapter, &std::env::current_exe()?, &settings::directory().join("rules"), scratch.path())?;
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
            let candidate = LinterRyoAdapter { executable: std::env::current_exe()?, rules: settings::directory().join("rules") };
            let case_scratch = tempfile::tempdir()?;
            let _ = ryo_benchmark::outbound::suite::adapters::local::adapter::verify_cases(&reference, &ExecutionProcessAdapter, &std::env::current_exe()?, &settings::directory().join("rules"), case_scratch.path())?;
            let suite: Suite = ryo_benchmark::outbound::suite::adapters::local::schemas::suite(&std::fs::read_to_string(settings::directory().join("../../benchmark.toml"))?)?;
            if suite.language != "python" || !(2..=3).contains(&suite.projects.len()) { return Err(Error::Invalid("Python suite requires2–3 projects".into())); }
            for project in suite.projects {
                let mut workspace = WorkspaceGitAdapter.prepare(&project, &cache.join("projects"))?;
                let files = reference.files(workspace.root())?;
                let tracked = WorkspaceGitAdapter.tracked_files(&workspace)?;
                if files.iter().any(|file| !tracked.contains(file)) { return Err(Error::Invalid("Ruff discovered untracked source".into())); }
                let request = RunRequest { root: workspace.root().to_owned(), files, cache: cache.join("runs").join(&project.name) };
                if request.cache.exists() { return Err(Error::Invalid("run cache already exists: use a fresh cache for a cold trial".into())); }
                let cold = measure(&audit, &reference, &candidate, &ExecutionProcessAdapter, &request)?;
                let warm = measure(&audit, &reference, &candidate, &ExecutionProcessAdapter, &request)?;
                compare(&cold.candidate.report, &warm.candidate.report).map_err(|difference| Error::Invalid(format!("warm vs cold mismatch: {difference:?}")))?;
                WorkspaceGitAdapter.apply_patch(&mut workspace, &settings::directory().join("../..").join(project.patch))?;
                let edited = measure(&audit, &reference, &candidate, &ExecutionProcessAdapter, &request);
                let clean_request = RunRequest { cache: cache.join("clean-edited").join(&project.name), ..request.clone() };
                let clean = execute(&candidate, &ExecutionProcessAdapter, &clean_request);
                WorkspaceGitAdapter.reset(&mut workspace)?;
                let edited = edited?;
                let clean = clean?;
                compare(&edited.candidate.report, &clean.report).map_err(|difference| Error::Invalid(format!("edited warm vs clean mismatch: {difference:?}")))?;
                println!("{} cold={:?}/{:?} warm={:?}/{:?} edited={:?}/{:?} (reference/ryo wall; no CPU/RSS claim)", project.name, cold.reference.wall, cold.candidate.wall, warm.reference.wall, warm.candidate.wall, edited.reference.wall, edited.candidate.wall);
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
