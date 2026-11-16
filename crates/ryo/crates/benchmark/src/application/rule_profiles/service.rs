//! Verify a bench rule by rule when its rules carry settings profiles of their own.
use crate::outbound::inventory::interface::InventoryPort;
use crate::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactText},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;
use std::path::Path;

/// Run `verify` over the bench at `source`, each rule under its own settings profile.
///
/// A bench whose rules all run under the bench's profile is verified whole: `verify` is given `source` once. When any rule
/// has a `profiles/<id>` profile of its own, every rule with a definition is staged as the registration stages it
/// ([`stage_rule_profile`](crate::application::registration::service::stage_rule_profile), the one function verify-inventory
/// registers through) and `verify` is given each stage; the failures of every rule are reported together. Answers the sum of
/// the counts `verify` answered.
///
/// # Errors
/// Refuses an unreadable bench, a rule that cannot be staged, a shard over per-rule profiles (a shard partitions one
/// profile's cases), and any failure of `verify`.
pub fn verify_each(
    suite: &dyn SuitePort,
    inventory: &dyn InventoryPort,
    source: &Path,
    shard: ShardPartition,
    scratch: &str,
    verify: &mut dyn FnMut(&Path) -> Result<usize, Error>,
) -> Result<usize, Error> {
    let tool = suite.profile(ArtifactPath(source))?;
    let mut own = false;
    let mut rules = Vec::new();
    for rule in &tool.defaults {
        own |= suite.settings_directory(ArtifactPath(source), rule)? != source;
        let defined = suite
            .rule_query(
                ArtifactPath(&source.join(crate::domain::suite::constants::RULES)),
                rule,
            )?
            .is_some();
        if defined {
            rules.push(rule);
        }
    }
    if !own || rules.is_empty() {
        return verify(source);
    }
    if shard != ShardPartition::All {
        return Err(Error::Invalid(super::constants::SHARD_ACROSS_PROFILES.into()));
    }
    let scratch = suite.scoped_directory(ArtifactPath(Path::new("")), ArtifactText(scratch))?;
    let mut count = 0;
    let mut failures = Vec::new();
    for rule in rules {
        let stage = crate::application::registration::service::stage_rule_profile(
            suite,
            inventory,
            source,
            rule,
            scratch.path(),
        )?;
        match verify(&stage) {
            Ok(checked) => count += checked,
            Err(error) => failures.push(format!("{rule}: {error}")),
        }
    }
    if failures.is_empty() {
        Ok(count)
    } else {
        Err(Error::Invalid(
            failures.join(crate::domain::suite::constants::LINE_SEPARATOR),
        ))
    }
}
