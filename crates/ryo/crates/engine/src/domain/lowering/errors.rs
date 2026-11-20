//! Why the lowering refuses a rule.

use super::types::PlanShape;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Why a rule's plan has no programs.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::errors::LoweringError;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// assert_eq!(LoweringError::Budget { rule: SelectedRuleOrdinal(2) }.to_string(), "rule SelectedRuleOrdinal(2) not loaded: program over budget");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoweringError {
    /// The plan has a shape the lowering cannot express.
    #[error("rule {rule:?} not loaded: {shape:?}")]
    Unlowerable {
        /// The rule.
        rule: SelectedRuleOrdinal,
        /// What of its plan the lowering cannot express.
        shape: PlanShape,
    },
    /// The rule's program exceeds the budget: 4,096 instructions or 50 ms of lowering.
    #[error("rule {rule:?} not loaded: program over budget")]
    Budget {
        /// The rule.
        rule: SelectedRuleOrdinal,
    },
}
