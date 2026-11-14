//! The `FrontierPlace` value.

use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_values::domain::plan_parts::types::InputNumber;

/// One entry of a rule's frontier, by the rule and the entry's place in its plan's frontier: what each file hands over to the
/// rule's steps after the files. Two rules' entries are two places even when the steps behind them are one node, because each
/// rule's files are its own (skipped, inapplicable and refused files differ).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrontierPlace {
    /// The rule.
    pub rule: RuleOrdinal,
    /// The entry's place in the rule's plan frontier.
    pub input: InputNumber,
}

impl FrontierPlace {
    /// Entry `input` of `rule`'s plan frontier.
    #[must_use]
    pub const fn of(rule: RuleOrdinal, input: InputNumber) -> Self {
        Self { rule, input }
    }
}
