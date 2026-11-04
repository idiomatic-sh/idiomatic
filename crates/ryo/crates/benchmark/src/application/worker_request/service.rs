//! Classify the sealed primary inventory once before the outbound SDK execution boundary.
use super::{constants, types::PreparedRequest};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::suite::service::{context_input_path, declares_missing};
use crate::domain::suite::types::{ProjectContextMode, WorkerRequest};
use crate::outbound::engine::types::PreparedMissingInputs;
use std::collections::BTreeSet;

/// Prepare a complete inventory, keeping declared missing paths distinct from readable files.
///
/// # Errors
/// Refuses a mismatched mode, source seal, or requested path inventory.
pub fn prepare(request: WorkerRequest) -> Result<PreparedRequest, Error> {
    crate::domain::suite::service::validate_selected_rules(request.selected_rules.as_deref())?;
    if let Some(context) = &request.rule_context {
        crate::domain::suite::rule_context::check(context, &request.files)?;
        crate::domain::suite::rule_context::check_selection(
            context,
            request.selected_rules.as_deref(),
        )?;
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
        missing: PreparedMissingInputs { selected_missing },
    })
}
