//! The prepared engine and the SDK fixtures every engine proof shares, below the proofs that use them.
use ryo_benchmark::outbound::engine::{factory, interface::EnginePort};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark::outbound::context::interface::ContextPort as _;
use ryo_benchmark_contracts::domain::reports::types::ProjectContext;
use ryo_benchmark_contracts::domain::requests::types::{ContextSeal, WorkerRequest};
use std::collections::BTreeMap;

/// Reuse the production worker preparation boundary before exercising the SDK adapter.
pub(super) trait PreparedEngine: EnginePort {
    fn run_prepared(
        &self,
        request: WorkerRequest,
    ) -> Result<ryo_benchmark_contracts::domain::reports::types::RunReport, Error>;
}

impl<T: EnginePort + ?Sized> PreparedEngine for T {
    fn run_prepared(
        &self,
        request: WorkerRequest,
    ) -> Result<ryo_benchmark_contracts::domain::reports::types::RunReport, Error> {
        let prepared = ryo_benchmark::application::worker_request::service::prepare(request)?;
        let context = ryo_benchmark::outbound::context::factory::local();
        if let ContextSeal::Sealed(manifest) = &prepared.request.context_manifest {
            context.validate(manifest, &prepared.request.root)?;
        }
        let session = self.load(&prepared.request)?;
        let configured = ryo_benchmark::application::worker_request::service::configure(
            &prepared.request,
            session,
        )?;
        let run = configured.execute()?;
        let mut report = run.report;
        if let ContextSeal::Sealed(manifest) = &prepared.request.context_manifest {
            report.project_context = ProjectContext::Observed(context.receipt(
                manifest,
                &prepared.request.root,
                run.dependencies,
            )?);
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
) -> BTreeMap<ryo_benchmark_contracts::domain::requests::types::QueryModuleName, std::path::PathBuf> {
    modules
        .into_iter()
        .map(|(name, path)| (ryo_benchmark_contracts::domain::requests::types::QueryModuleName(name), path))
        .collect()
}

pub(super) fn suppress_modules() -> BTreeMap<String, std::path::PathBuf> {
    BTreeMap::from([
        (
            "python/ruff/suppress-matches".to_owned(),
            "fragments/suppress-matches.ryoql".into(),
        ),
        (
            "python/ruff/noqa-line-map".to_owned(),
            std::path::PathBuf::from("fragments/noqa-line-map.ryoql"),
        ),
        (
            "python/ruff/noqa-locations".to_owned(),
            std::path::PathBuf::from("fragments/noqa-locations.ryoql"),
        ),
        (
            "python/ruff/noqa-additional-locations".to_owned(),
            std::path::PathBuf::from("fragments/noqa-additional-locations.ryoql"),
        ),
        (
            "python/ruff/suppression-edges".to_owned(),
            std::path::PathBuf::from("fragments/suppression-edges.ryoql"),
        ),
        (
            "python/ruff/suppress".to_owned(),
            std::path::PathBuf::from("fragments/suppress.ryoql"),
        ),
        (
            "python/ruff/suppress-directives".to_owned(),
            std::path::PathBuf::from("fragments/suppress-directives.ryoql"),
        ),
    ])
}

pub(super) fn default_frontend_projection() -> Result<ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection, Error>
{
    Ok(ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
        rule: ryo::linter::RuleId::try_from("invalid-syntax")
            .map_err(|error| Error::Invalid(error.to_string()))?,
        query: ryo::query::RyoQlQuery::try_from(
            crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
        )
        .map_err(|error| Error::Invalid(error.to_string()))?,
    })
}

pub(super) fn custom_frontend_projection() -> Result<ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection, Error> {
    Ok(ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
        rule: ryo::linter::RuleId::try_from("parser-check")
            .map_err(|error| Error::Invalid(error.to_string()))?,
        query: ryo::query::RyoQlQuery::try_from(r#"let diagnostics = select id, file, span from "syntax-error" where message = "nonlocal declaration not allowed at module level";
emit diagnostics subject file key(id);
"#)
            .map_err(|error| Error::Invalid(error.to_string()))?,
    })
}
