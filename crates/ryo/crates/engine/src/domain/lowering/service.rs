//! The `lowering` service: each selected rule's plan into its programs (scope-engine2 2.5, track L1). Until the lowering expresses
//! a step, it refuses every rule at its first step, so `load` accepts no rule and the harness counts every rule refused.

use super::errors::LoweringError;
use super::types::{PlanNode, PlanShape, SelectedRules};
use crate::domain::programs::types::Programs;
use ryo_values::domain::descriptions::types::KindVocabulary;

/// The programs of the selected rules, or the first rule whose plan the lowering cannot express.
///
/// # Errors
///
/// `Unlowerable` naming the first rule and the step of its plan the lowering does not express.
pub(crate) fn lowered(
    rules: SelectedRules<'_>,
    _kinds: &KindVocabulary,
) -> Result<Programs, LoweringError> {
    match rules.0.first() {
        Some(first) => Err(LoweringError::Unlowerable {
            rule: first.rule,
            shape: PlanShape::Step(PlanNode(0)),
        }),
        None => Ok(Programs::default()),
    }
}
