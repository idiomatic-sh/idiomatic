//! Verification of a bench under the settings profile of each rule.
use crate::outbound::suite::types::ArtifactText;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::path::Path;
/// Run a verification over a bench under each rule's own settings profile.
pub trait VerifyRuleProfilesUseCase {
    /// A bench whose rules all run under the bench's profile is verified whole: `verify` is given `source` once. When any
    /// rule has a `profiles/<id>` profile of its own, every rule with a definition is staged as the registration stages it
    /// and `verify` is given each stage; the failures of every rule are reported together. Answers the sum of the counts
    /// `verify` answered.
    ///
    /// # Errors
    /// Refuses an unreadable bench, a rule that cannot be staged, a shard over per-rule profiles (a shard partitions one
    /// profile's cases), and any failure of `verify`.
    fn verify_rule_profiles(
        &self,
        source: &Path,
        shard: ShardPartition,
        scratch: ArtifactText<'_>,
        verify: &mut dyn FnMut(&Path) -> Result<CaseCount, ReportsError>,
    ) -> Result<CaseCount, ReportsError>;
}
