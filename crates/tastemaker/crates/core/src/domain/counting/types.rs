//! The types of the `counting` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::findings::types::Finding;
use crate::domain::scopes::types::{CountCheck, PairCheck};

/// What the counts of a placement come to: the counts and pairs that wait for a condition, and the findings of the
/// pairs found now.
#[derive(Clone, Debug, Default)]
pub struct Counted {
    /// The counts to judge once the conditions are answered.
    pub checks: Vec<CountCheck>,
    /// The pairs to report once the conditions are answered.
    pub pairs: Vec<PairCheck>,
    /// The findings of the pairs of root entries, found now.
    pub findings: Vec<Finding>,
}
