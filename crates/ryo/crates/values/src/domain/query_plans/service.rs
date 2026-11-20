//! The `query_plans` service: free functions named by verb over a whole compiled query, with no IO and no ports.

use super::types::{Plan, RuleReads};
use crate::domain::plan_parts::types::Input;
use crate::domain::plans::service::{inputs_of, named};

/// The relations `plan`'s steps read, each once, in the order a step first reads it: the one walk of a rule's read set.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_plans::service::reads_of;
/// use ryo_values::domain::query_plans::types::Plan;
/// fn reads(plan: &Plan) -> usize {
///     reads_of(plan).0.len()
/// }
/// ```
#[must_use]
pub fn reads_of(plan: &Plan) -> RuleReads {
    let mut reads = Vec::new();
    for step in &plan.steps {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Relation(relation) = *input
                && !reads.contains(&relation)
            {
                reads.push(relation);
            }
        }
    }
    RuleReads(reads.into_boxed_slice())
}
