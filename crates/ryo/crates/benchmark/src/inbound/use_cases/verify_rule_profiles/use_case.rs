//! Readable orchestration, borrowing the actual injected port owners.
use super::interface::VerifyRuleProfilesUseCase;
use crate::outbound::inventory::interface::InventoryPort;
use crate::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactText},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::path::Path;
/// verify_rule_profiles with required borrowed boundaries.
pub struct VerifyRuleProfilesUseCaseImpl<'a> {
    suite: &'a dyn SuitePort,
    inventory: &'a dyn InventoryPort,
}
impl<'a> VerifyRuleProfilesUseCaseImpl<'a> {
    /// Borrow the boundaries already composed by the caller.
    #[must_use]
    pub fn new(suite: &'a dyn SuitePort, inventory: &'a dyn InventoryPort) -> Self {
        Self { suite, inventory }
    }
}
impl VerifyRuleProfilesUseCase for VerifyRuleProfilesUseCaseImpl<'_> {
    fn verify_rule_profiles(
        &self,
        source: &Path,
        shard: ShardPartition,
        scratch: ArtifactText<'_>,
        verify: &mut dyn FnMut(&Path) -> Result<CaseCount, Error>,
    ) -> Result<CaseCount, Error> {
        let tool = self.suite.profile(ArtifactPath(source))?;
        let rules_directory = source.join(crate::domain::suite::constants::RULES);
        let mut own = false;
        let mut rules = Vec::new();
        for rule in &tool.defaults {
            own |= self.suite.settings_directory(ArtifactPath(source), rule)? != source;
            if self
                .suite
                .rule_query(ArtifactPath(&rules_directory), rule)?
                .is_some()
            {
                rules.push(rule);
            }
        }
        if !own || rules.is_empty() {
            return verify(source);
        }
        if shard != ShardPartition::All {
            return Err(Error::Invalid(
                super::constants::SHARD_ACROSS_PROFILES.into(),
            ));
        }
        let scratch = self.suite.scoped_directory(
            ArtifactPath(Path::new(super::constants::SCRATCH_ROOT)),
            scratch,
        )?;
        let mut total = CaseCount::default();
        let mut failures = Vec::new();
        for rule in rules {
            let stage = crate::application::registration::service::stage_rule_profile(
                self.suite,
                self.inventory,
                source,
                rule,
                scratch.path(),
            )?;
            match verify(&stage) {
                Ok(checked) => total = CaseCount(total.0 + checked.0),
                Err(error) => failures.push(format!("{rule}: {error}")),
            }
        }
        if failures.is_empty() {
            Ok(total)
        } else {
            Err(Error::Invalid(
                failures.join(crate::domain::suite::constants::LINE_SEPARATOR),
            ))
        }
    }
}
