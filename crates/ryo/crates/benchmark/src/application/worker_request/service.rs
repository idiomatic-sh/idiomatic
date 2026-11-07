//! Classify the sealed primary inventory once before the outbound SDK execution boundary.
use super::{constants, types::PreparedWorkerRequest};
use crate::domain::suite::service::context_input_path;
use crate::domain::suite::service::declares_missing;
use crate::outbound::engine::types::PreparedMissingInputs;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeSet;

/// Prepare a complete inventory, keeping declared missing paths distinct from readable files.
///
/// # Errors
/// Refuses a mismatched mode, source seal, or requested path inventory.
pub fn prepare(request: WorkerRequest) -> Result<PreparedWorkerRequest, Error> {
    let missing = classify(&request)?;
    Ok(PreparedWorkerRequest { request, missing })
}
fn classify(request: &WorkerRequest) -> Result<PreparedMissingInputs, Error> {
    crate::domain::suite::service::validate_selected_rules(&request.selected_rules)?;
    let ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(selected_rules) = request.selected_rules.clone() else {
        return Err(Error::Invalid(
            constants::WORKER_REQUIRES_A_RESOLVED_DIAGNOSTIC_SELECT.into(),
        ));
    };
    if let ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Prepared(context) = &request.rule_context {
        crate::domain::rule_context::service::check(context, &request.files)?;
        crate::domain::rule_context::service::check_selection(context, &request.selected_rules)?;
    }

    if let ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(roots) = &request.project_search_roots {
        for root in roots {
            if !root.0.is_empty() {
                ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(root.0.as_ref()))?;
            }
        }
    }
    for entry in &request.given_project_inventory.entries {
        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(entry.path.0.as_ref()))?;
    }
    if request.given_project_inventory.completeness
        == ryo::linter::ProjectInventoryCompleteness::CompleteWithinDeclaredScope
        && request.project_search_roots == ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared
    {
        return Err(Error::Invalid(
            constants::COMPLETE_GIVEN_PROJECT_INVENTORY_REQUIRES_DE.into(),
        ));
    }
    if request.project_context_mode == ProjectContextMode::ListedDisk
        && (!request.given_project_inventory.entries.is_empty()
            || request.given_project_inventory.completeness
                != ryo::linter::ProjectInventoryCompleteness::Partial)
    {
        return Err(Error::Invalid(
            constants::GIVEN_PROJECT_INVENTORY_CANNOT_OVERRIDE_A.into(),
        ));
    }
    let selected_missing = match (request.project_context_mode, &request.context_manifest) {
        (ProjectContextMode::Isolated, ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed) => BTreeSet::new(),
        (ProjectContextMode::ListedDisk, ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(manifest)) => {
            let requested: BTreeSet<_> = request
                .files
                .iter()
                .map(|file| context_input_path(&manifest.logical_root, file))
                .collect();
            let sealed_sources: BTreeSet<_> = manifest.source_files.keys().cloned().collect();
            let selected_missing: BTreeSet<_> = request
                .files
                .iter()
                .filter(|file| {
                    declares_missing(manifest, file)
                        == crate::domain::suite::types::ContextAbsence::Sealed
                })
                .cloned()
                .collect();
            let missing_in_context: BTreeSet<_> = selected_missing
                .iter()
                .map(|file| context_input_path(&manifest.logical_root, file))
                .collect();
            if !sealed_sources.is_subset(&requested)
                || requested != sealed_sources.union(&missing_in_context).cloned().collect()
            {
                return Err(Error::Invalid(constants::CONTEXT_SOURCE_MISMATCH.into()));
            }
            selected_missing
        }
        _ => {
            return Err(Error::Invalid(constants::CONTEXT_MODE_MISMATCH.into()));
        }
    };
    Ok(PreparedMissingInputs {
        selected_missing,
        selected_rules,
    })
}

/// Configure the single loaded SDK rule map through pure domain policy.
///
/// # Errors
/// Refuses missing selected/producers, invalid frontend scope or invalid rule context.
pub fn configure<'a>(
    expected: &WorkerRequest,
    mut session: crate::outbound::engine::types::loaded_engine_session::value::LoadedEngineSession<
        'a,
    >,
) -> Result<
    crate::outbound::engine::types::configured_engine_session::value::ConfiguredEngineSession<'a>,
    Error,
> {
    use crate::domain::candidate_rules::service as candidate_rules;
    let request = session.request();
    if !std::ptr::eq(expected, request) {
        return Err(Error::Invalid(
            constants::LOADED_ENGINE_SESSION_BELONGS_TO_ANOTHER.into(),
        ));
    }
    let missing = classify(request)?;
    let frontend = session.frontend_rule().cloned();
    let diagnostic_rules = {
        let rules = session.rules_mut();
        candidate_rules::retain_producers(&missing.selected_rules, frontend.as_ref(), rules)?
    };
    if let Some(frontend) = &frontend {
        {
            let rules = session.rules_mut();
            candidate_rules::exclude_missing(rules, frontend, &missing.selected_missing)?;
        }
        let configuration_files = session.configuration_files()?;
        let rules = session.rules_mut();
        candidate_rules::exclude_configuration(rules, frontend, &configuration_files)?;
    }
    let mut selected_rules = diagnostic_rules.clone();
    if let Some(frontend) = frontend {
        selected_rules.insert(frontend);
    }
    let configured = {
        let rules = session.rules_mut();
        let selected_pairs = diagnostic_rules.len().saturating_mul(request.files.len());
        candidate_rules::project(
            request,
            rules,
            ryo_benchmark_contracts::domain::reports::types::PairCount(selected_pairs),
        )?
    };
    let context = crate::outbound::engine::types::EngineConfiguration {
        selected_missing: missing.selected_missing,
        selected_rules,
        selected_pairs: crate::outbound::engine::types::SelectedPairs(configured.selected_pairs.0),
        hosted: configured.hosted,
    };
    Ok(crate::outbound::engine::types::configured_engine_session::value::ConfiguredEngineSession::new(session, context))
}
