//! Classify the sealed primary inventory once before the outbound SDK execution boundary.
use super::{constants, types::PreparedRequest};
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
pub fn prepare(request: WorkerRequest) -> Result<PreparedRequest, Error> {
    crate::domain::suite::service::validate_selected_rules(request.selected_rules.as_deref())?;
    let selected_rules = request
        .selected_rules
        .clone()
        .ok_or_else(|| Error::Invalid("worker requires a resolved diagnostic selection".into()))?;
    if let Some(context) = &request.rule_context {
        crate::domain::suite::rule_context::check(context, &request.files)?;
        crate::domain::suite::rule_context::check_selection(
            context,
            request.selected_rules.as_deref(),
        )?;
    }

    if let Some(roots) = &request.project_search_roots {
        for root in roots {
            if !root.0.is_empty() {
                crate::domain::suite::service::relative_path(root.0.as_ref())?;
            }
        }
    }
    for entry in &request.given_project_inventory.entries {
        crate::domain::suite::service::relative_path(entry.path.0.as_ref())?;
    }
    if request.given_project_inventory.completeness
        == ryo::linter::ProjectInventoryCompleteness::CompleteWithinDeclaredScope
        && request.project_search_roots.is_none()
    {
        return Err(Error::Invalid(
            "complete Given project inventory requires declared search roots".into(),
        ));
    }
    if request.project_context_mode == ProjectContextMode::ListedDisk
        && (!request.given_project_inventory.entries.is_empty()
            || request.given_project_inventory.completeness
                != ryo::linter::ProjectInventoryCompleteness::Partial)
    {
        return Err(Error::Invalid(
            "Given project inventory cannot override a sealed disk context".into(),
        ));
    }
    let selected_missing = match (request.project_context_mode, &request.context_manifest) {
        (ProjectContextMode::Isolated, None) => BTreeSet::new(),
        (ProjectContextMode::ListedDisk, Some(manifest)) => {
            let requested: BTreeSet<_> = request
                .files
                .iter()
                .map(|file| context_input_path(&manifest.logical_root, file))
                .collect();
            let sealed_sources: BTreeSet<_> = manifest.source_files.keys().cloned().collect();
            let selected_missing: BTreeSet<_> = request
                .files
                .iter()
                .filter(|file| declares_missing(manifest, file))
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
    Ok(PreparedRequest {
        request,
        missing: PreparedMissingInputs {
            selected_missing,
            selected_rules,
        },
    })
}
