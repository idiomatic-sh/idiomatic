//! Admit authored cases and compose physical materialization for the verifier.
use crate::domain::suite::{rule_context, service as policy};
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::{
    context::types::project_context_mode::value::ProjectContextMode, requests::types::RunRequest,
    workloads::types::PreparedCase,
};
use std::path::Path;
/// Load, admit and materialize the independent sources through the physical suite port.
pub(crate) fn cases(
    suite: &dyn SuitePort,
    rules: &Path,
    scratch: &Path,
) -> Result<Vec<PreparedCase>, Error> {
    let profile_root = rules
        .parent()
        .ok_or_else(|| Error::Invalid(super::constants::MISSING_PROFILE_PARENT.into()))?;
    let profile = match suite.profile(ArtifactPath(profile_root)) {
        Ok(profile) => Some(profile),
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let context_mode = profile
        .as_ref()
        .map_or(ProjectContextMode::Isolated, |tool| {
            tool.project_context_mode
        });
    let mut prepared = Vec::new();
    for rule in suite.rule_ids(ArtifactPath(rules))? {
        if profile
            .as_ref()
            .is_some_and(|tool| !policy::owns_cases(tool, &rule))
        {
            continue;
        }
        let cases = suite.rule_cases(ArtifactPath(rules), &rule)?;
        policy::validate_case_sources(&cases)?;
        if profile
            .as_ref()
            .is_some_and(|tool| tool.file_observation.is_some())
            && cases.cases.iter().any(|case| {
                case.file_metadata
                    .as_ref()
                    .is_none_or(|values| values.len() != case.files.len())
            })
        {
            return Err(Error::Invalid(super::constants::REQUIRED_METADATA.into()));
        }
        match suite.rule_query(ArtifactPath(rules), &rule)? {
            Some(query) if !query.trim().is_empty() => {}
            Some(_) => return Err(Error::Invalid(format!("empty query for {rule}"))),
            None => continue,
        }
        policy::validate_cases(&rule, &cases)?;
        for (index, mut case) in cases.cases.into_iter().enumerate() {
            let project_search_roots = case.project_search_roots.take().or_else(|| {
                profile
                    .as_ref()
                    .and_then(|tool| tool.project_search_roots.clone())
            });
            let given_project_inventory = case.given_project_inventory.take().unwrap_or_default();
            if given_project_inventory.completeness
                == ryo::linter::ProjectInventoryCompleteness::CompleteWithinDeclaredScope
                && project_search_roots.is_none()
            {
                return Err(Error::Invalid(
                    super::constants::COMPLETE_GIVEN_ROOTS.into(),
                ));
            }
            if context_mode == ProjectContextMode::ListedDisk
                && (!given_project_inventory.entries.is_empty()
                    || given_project_inventory.completeness
                        != ryo::linter::ProjectInventoryCompleteness::Partial)
            {
                return Err(Error::Invalid(
                    super::constants::GIVEN_OVERRIDES_SEAL.into(),
                ));
            }
            if !matches!(
                (context_mode, &case.context),
                (ProjectContextMode::Isolated, None) | (ProjectContextMode::ListedDisk, Some(_))
            ) {
                return Err(Error::Invalid(super::constants::CONTEXT_MODE.into()));
            }
            let physical = suite.materialize_case(
                ArtifactPath(&scratch.join(rule.to_string()).join(index.to_string())),
                &case,
            )?;
            let rule_context = rule_context::resolve(
                profile.as_ref().and_then(|tool| tool.rule_context.as_ref()),
                case.rule_context.as_ref(),
            )?;
            let selected_rules = case.selected_rules.or_else(|| {
                rule_context
                    .as_ref()
                    .and_then(|_| profile.as_ref().map(|tool| tool.defaults.clone()))
            });
            suite.materialize_inventory(ArtifactPath(&physical.root), &given_project_inventory)?;
            let request = RunRequest {
                performance: case.performance,
                rule_context,
                selected_rules,
                rule_observations: case.rule_observations,
                project_search_roots,
                given_project_inventory,
                staged_sources: Some(case.files.keys().cloned().collect()),
                file_metadata: case.file_metadata,
                context_manifest: physical.context_manifest,
                root: physical.root,
                files: case
                    .requested_files
                    .unwrap_or_else(|| case.files.keys().cloned().collect()),
                state: scratch
                    .join(super::constants::CACHE)
                    .join(rule.to_string())
                    .join(index.to_string()),
            };
            let expected = policy::expected_report(&rule, request.files.clone(), case.expected);
            prepared.push(PreparedCase {
                reference_discrepancy: None,
                comparison: case.comparison,
                rule: rule.clone(),
                name: case.name,
                request,
                expected,
            });
        }
    }
    if prepared.is_empty() {
        return Err(Error::Invalid(super::constants::NO_EXECUTABLE_CASES.into()));
    }
    Ok(prepared)
}
