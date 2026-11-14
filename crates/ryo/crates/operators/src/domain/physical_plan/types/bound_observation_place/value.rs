//! The `BoundObservationPlace` value.

use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_values::domain::plan_parts::types::ObservationBindingId;

/// One bound observation of a rule, by the rule and the binding's place among its plan's observation inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoundObservationPlace {
    /// The rule.
    pub rule: RuleOrdinal,
    /// The binding's place among the rule's observation inputs.
    pub input: ObservationBindingId,
}

impl BoundObservationPlace {
    /// Bound observation `input` of `rule`.
    #[must_use]
    pub const fn of(rule: RuleOrdinal, input: ObservationBindingId) -> Self {
        Self { rule, input }
    }
}
