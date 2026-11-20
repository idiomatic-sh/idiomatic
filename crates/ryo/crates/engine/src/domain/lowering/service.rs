//! The `lowering` service: the selected rules' plans into one lowered pack (scope-engine3 2.6, track L1). Until the lowering
//! evaluates a step, it refuses every rule at its first step, so `load` accepts no rule and the harness counts every rule
//! refused.

use super::types::{LoadRefusal, LoweredRules, PlanNode, RefusedRule, SelectedRules};
use crate::domain::lowered::types::LoweredPack;
use crate::domain::statistics::types::Statistics;
use ryo_values::domain::descriptions::types::KindVocabulary;

/// The pack of the rules the lowering accepts, and each rule it refuses with the step it cannot evaluate or the count it cannot
/// write. No rule is refused for what it costs or for how large it is.
pub(crate) fn lowered(
    rules: SelectedRules<'_>,
    statistics: &Statistics,
    kinds: &KindVocabulary,
) -> LoweredRules {
    let _ = (statistics, kinds);
    LoweredRules {
        pack: LoweredPack::default(),
        refused: rules
            .0
            .iter()
            .map(|rule| RefusedRule {
                rule: rule.rule,
                refusal: LoadRefusal::Unlowerable(PlanNode(0)),
            })
            .collect(),
    }
}
