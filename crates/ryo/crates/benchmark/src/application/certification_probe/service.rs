//! Both contestants prepare one neutral request so a certificate can bind their invocation identities.
use super::constants;
use super::types::ProbedProfiles;
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, FileMetadataDeclaration, InputDeclaration,
    RuleContextPreparation, RunRequest, SearchRoots, StagedSources,
};
use std::path::Path;

/// Prepare the neutral probe request on both contestants.
///
/// # Errors
/// Propagates a contestant's preparation failure.
pub fn probe(
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    directory: &Path,
    scratch: &Path,
) -> Result<ProbedProfiles, Error> {
    let probe = RunRequest {
        performance: Default::default(),
        selected_rules: CohortSelection::Frozen,
        rule_observations: InputDeclaration::Undeclared,
        rule_context: RuleContextPreparation::Unconfigured,
        project_search_roots: SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: StagedSources::Unlisted,
        file_metadata: FileMetadataDeclaration::Undeclared,
        context_manifest: ContextSeal::Unsealed,
        root: directory.to_owned(),
        files: vec![ryo::linter::RelativePath(constants::PROFILE_PROBE.into())],
        state: scratch.join(constants::IDENTITY_STATE),
    };
    let profiles = [reference.prepare(&probe)?, candidate.prepare(&probe)?];
    Ok(ProbedProfiles { probe, profiles })
}
