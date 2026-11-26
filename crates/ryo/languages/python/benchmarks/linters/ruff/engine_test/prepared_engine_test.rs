//! The prepared engine and the SDK fixtures every engine proof shares, below the proofs that use them.
use benchmark::outbound::context::interface::ContextPort as _;
use benchmark::outbound::engine::interface::EnginePort;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::ProjectContext;
use benchmark_contracts::domain::requests::types::{ContextSeal, WorkerRequest};
use std::collections::BTreeMap;

/// Reuse the production worker preparation boundary before exercising the SDK adapter, bracketed as the
/// harness brackets the worker: `validate` before load and `verify_receipt` after the receipt.
pub(super) trait PreparedEngine: EnginePort {
    fn run_prepared(
        &self,
        request: WorkerRequest,
    ) -> Result<benchmark_contracts::domain::reports::types::RunReport, Error>;
}

impl<T: EnginePort + ?Sized> PreparedEngine for T {
    fn run_prepared(
        &self,
        request: WorkerRequest,
    ) -> Result<benchmark_contracts::domain::reports::types::RunReport, Error> {
        let prepared = benchmark::application::worker_request::service::prepare(request)?;
        let context = benchmark::outbound::context::factory::local();
        if let ContextSeal::Sealed(manifest) = &prepared.request.context_manifest {
            context.validate(manifest, &prepared.request.root)?;
        }
        let session = self.load(&prepared.request)?;
        let configured =
            benchmark::application::worker_request::service::configure(&prepared.request, session)?;
        let run = configured.execute()?;
        let mut report = run.report;
        if let ContextSeal::Sealed(manifest) = &prepared.request.context_manifest {
            let receipt = context.receipt(manifest, &prepared.request.root, run.dependencies)?;
            context.verify_receipt(manifest, &prepared.request.root, &receipt)?;
            report.project_context = ProjectContext::Observed(receipt);
        }
        Ok(report)
    }
}

pub(super) fn python_310() -> ryo::linter::Languages {
    ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(
            ryo_python::settings::PythonTargetVersion::Py310,
        ),
    )])
}

/// Explicit fixture diagnostics, independent of whatever query directories happen to exist.
pub(super) fn fixture_cohort(names: &[&str]) -> Vec<ryo::linter::RuleId> {
    names
        .iter()
        .map(|name| ryo::linter::RuleId::try_from(*name).unwrap())
        .collect()
}

/// The module registry of a worker request, keyed by logical module name.
pub(super) fn module_names(
    modules: BTreeMap<String, std::path::PathBuf>,
) -> BTreeMap<benchmark_contracts::domain::requests::types::QueryModuleName, std::path::PathBuf> {
    modules
        .into_iter()
        .map(|(name, path)| {
            (
                benchmark_contracts::domain::requests::types::QueryModuleName(name),
                path,
            )
        })
        .collect()
}

/// The fragments a rule that suppresses by `# noqa` imports, with every fragment those import in turn: each module's name and where its
/// file is, below the crate. The set follows the fragments themselves, so a fragment added or removed is added or removed here.
pub(super) fn suppress_modules() -> BTreeMap<String, std::path::PathBuf> {
    fragments_closure(vec![
        "python/ruff/suppress-matches".to_owned(),
        "python/ruff/noqa-suppressions-with-additional-lines".to_owned(),
    ])
}

/// The modules a query text imports.
pub(super) fn imports_of(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.strip_prefix("import ").and_then(|rest| rest.split('"').nth(1)))
        .map(str::to_owned)
        .collect()
}

/// The fragments named in `pending` with every fragment those import in turn: each module's name and where its file is, below the crate.
pub(super) fn fragments_closure(mut pending: Vec<String>) -> BTreeMap<String, std::path::PathBuf> {
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut modules = BTreeMap::new();
    while let Some(name) = pending.pop() {
        if modules.contains_key(&name) {
            continue;
        }
        let path = std::path::PathBuf::from(format!(
            "fragments/{}.ryoql",
            name.strip_prefix("python/ruff/").unwrap_or(&name)
        ));
        let text = std::fs::read_to_string(source.join(&path))
            .unwrap_or_else(|error| panic!("the fragment {name} at {}: {error}", path.display()));
        pending.extend(imports_of(&text));
        let _first = modules.insert(name, path);
    }
    modules
}

pub(super) fn default_frontend_projection()
-> Result<benchmark::outbound::engine::types::FrontendDiagnosticProjection, Error> {
    Ok(
        benchmark::outbound::engine::types::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from("invalid-syntax")
                .map_err(|error| Error::Invalid(error.to_string()))?,
            query: ryo::query::RyoQlQuery::try_from(
                crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
            )
            .map_err(|error| Error::Invalid(error.to_string()))?,
        },
    )
}

pub(super) fn custom_frontend_projection()
-> Result<benchmark::outbound::engine::types::FrontendDiagnosticProjection, Error> {
    Ok(benchmark::outbound::engine::types::FrontendDiagnosticProjection {
        rule: ryo::linter::RuleId::try_from("parser-check")
            .map_err(|error| Error::Invalid(error.to_string()))?,
        query: ryo::query::RyoQlQuery::try_from(r#"let diagnostics = select id, file, span from "syntax-error" where message = "nonlocal declaration not allowed at module level";
emit diagnostics subject file key(id);
"#)
            .map_err(|error| Error::Invalid(error.to_string()))?,
    })
}
