//! The constants of the `encoding` concept.

use crate::domain::lowering::types::PlanNode;

/// The step the encoding's own fault names: the rule's last. The encoding does not hold a rule's plan, so it names the greatest
/// place and `load` holds it to the plan's last step.
pub(crate) const LAST_STEP: PlanNode = PlanNode(u32::MAX);
