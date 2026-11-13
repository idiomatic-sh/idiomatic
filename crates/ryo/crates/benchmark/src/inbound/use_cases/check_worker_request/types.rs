//! What the executable that composes a worker tells the identity check to expect.
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, ParserConfigurationName, RuleLabel,
};
use std::collections::BTreeMap;

/// The identity a worker request must carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerExpectation {
    /// The parser configuration the executable is linked with.
    pub parser_configuration: ParserConfigurationName,
    /// The pinned phase policy of every rule the language has pinned.
    pub pinned_applicability: BTreeMap<RuleLabel, ApplicabilityPolicy>,
}
