//! Admit authored cases and compose physical materialization for the verifier.
use crate::domain::rule_context::service as rule_context;
use crate::domain::suite::service as policy;
use crate::outbound::context::interface::ContextPort;
use crate::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, MaterializedContext},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::{
    context::types::project_context_mode::value::ProjectContextMode, requests::types::RunRequest,
    workloads::types::PreparedCase,
};
use std::path::Path;
/// Load, admit and materialize the independent sources through the physical suite port.
pub(crate) fn cases(
    suite: &dyn SuitePort,
    context: &dyn ContextPort,
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
    let undeclared_context = ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Undeclared;
    let mut prepared = Vec::new();
    for rule in suite.rule_ids(ArtifactPath(rules))? {
        if profile
            .as_ref()
            .is_some_and(|tool| {
                policy::owns_cases(tool, &rule) == crate::domain::suite::types::CaseOwnership::Supports
            })
        {
            continue;
        }
        let cases = suite.rule_cases(ArtifactPath(rules), &rule)?;
        policy::validate_case_sources(&cases)?;
        if profile
            .as_ref()
            .is_some_and(|tool| tool.file_observation != ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared)
            && cases.cases.iter().any(|case| match &case.file_metadata {
                ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Declared(values) => values.len() != case.files.len(),
                ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared => true,
            })
        {
            return Err(Error::Invalid(super::constants::REQUIRED_METADATA.into()));
        }
        match suite.rule_query(ArtifactPath(rules), &rule)? {
            Some(query) if !query.0.trim().is_empty() => {}
            Some(_) => return Err(Error::Invalid(format!("empty query for {rule}"))),
            None => continue,
        }
        policy::validate_cases(&rule, &cases)?;
        for (index, mut case) in cases.cases.into_iter().enumerate() {
            let project_search_roots = match std::mem::replace(
                &mut case.project_search_roots,
                ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            ) {
                ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared => profile.as_ref().map_or(
                    ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
                    |tool| tool.project_search_roots.clone(),
                ),
                declared => declared,
            };
            let given_project_inventory = match std::mem::replace(
                &mut case.given_project_inventory,
                ryo_benchmark_contracts::domain::workloads::types::InventoryDeclaration::Undeclared,
            ) {
                ryo_benchmark_contracts::domain::workloads::types::InventoryDeclaration::Declared(inventory) => inventory,
                ryo_benchmark_contracts::domain::workloads::types::InventoryDeclaration::Undeclared => Default::default(),
            };
            if given_project_inventory.completeness
                == ryo::linter::ProjectInventoryCompleteness::CompleteWithinDeclaredScope
                && project_search_roots == ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared
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
                (ProjectContextMode::Isolated, ryo_benchmark_contracts::domain::workloads::types::ContextDeclaration::Undeclared)
                    | (ProjectContextMode::ListedDisk, ryo_benchmark_contracts::domain::workloads::types::ContextDeclaration::Declared(_))
            ) {
                return Err(Error::Invalid(super::constants::CONTEXT_MODE.into()));
            }
            let boundary = scratch.join(rule.to_string()).join(index.to_string());
            let sealed = match &case.context {
                ryo_benchmark_contracts::domain::workloads::types::ContextDeclaration::Declared(
                    declared,
                ) => MaterializedContext::Sealed(context.materialize(
                    &boundary,
                    declared,
                    &case.files,
                )?),
                ryo_benchmark_contracts::domain::workloads::types::ContextDeclaration::Undeclared => {
                    MaterializedContext::Isolated
                }
            };
            let physical = suite.materialize_case(ArtifactPath(&boundary), &case, &sealed)?;
            let rule_context = rule_context::resolve(
                profile
                    .as_ref()
                    .map_or(&undeclared_context, |tool| &tool.rule_context),
                &case.rule_context,
            )?;
            let selected_rules = match case.selected_rules {
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen => match (&rule_context, &profile) {
                    (ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Prepared(_), Some(tool)) => {
                        ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(tool.defaults.clone())
                    }
                    _ => ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
                },
                explicit => explicit,
            };
            suite.materialize_inventory(ArtifactPath(&physical.root), &given_project_inventory)?;
            let request = RunRequest {
                performance: case.performance,
                rule_context,
                selected_rules,
                rule_observations: case.rule_observations,
                project_search_roots,
                given_project_inventory,
                staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Listed(case.files.keys().cloned().collect()),
                file_metadata: case.file_metadata,
                context_manifest: match sealed {
                    MaterializedContext::Isolated => {
                        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed
                    }
                    MaterializedContext::Sealed(manifest) => {
                        ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(manifest)
                    }
                },
                root: physical.root,
                files: match case.requested_files {
                    ryo_benchmark_contracts::domain::workloads::types::RequestedFiles::Explicit(files) => files,
                    ryo_benchmark_contracts::domain::workloads::types::RequestedFiles::Legacy => case.files.keys().cloned().collect(),
                },
                state: scratch
                    .join(super::constants::CACHE)
                    .join(rule.to_string())
                    .join(index.to_string()),
            };
            let expected = policy::expected_report(&rule, request.files.clone(), case.expected);
            prepared.push(PreparedCase {
                reference_discrepancy: ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees,
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
