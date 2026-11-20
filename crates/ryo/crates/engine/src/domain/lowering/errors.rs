//! Why the lowering, or the encoding after it, refuses a rule.

use super::types::LoadRefusal;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Why a rule has no programs: the rule, and what of it cannot be evaluated or written. One shape for the lowering and the
/// encoding; `load` answers it as the rule's `RefusedRule`.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::errors::LoweringError;
/// use ryo_engine::domain::lowering::types::{LoadRefusal, PastWidth, PlanNode};
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let rule = SelectedRuleOrdinal(2);
/// assert_eq!(
///     LoweringError::Refused { rule, refusal: LoadRefusal::Unlowerable(PlanNode(4)) }.to_string(),
///     "rule SelectedRuleOrdinal(2) not loaded: step PlanNode(4) cannot be evaluated"
/// );
/// assert_eq!(
///     LoweringError::Refused { rule, refusal: LoadRefusal::Budget(PastWidth::TableColumns) }.to_string(),
///     "rule SelectedRuleOrdinal(2) not loaded: more than 65,535 columns in one table"
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoweringError {
    /// The rule is not loaded.
    #[error("rule {rule:?} not loaded: {refusal}")]
    Refused {
        /// The rule.
        rule: SelectedRuleOrdinal,
        /// Why.
        refusal: LoadRefusal,
    },
}
