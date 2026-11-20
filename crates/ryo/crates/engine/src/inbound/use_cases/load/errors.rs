//! Why `load` refuses.

use crate::domain::lowering::types::PlanShape;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Why a rule set could not be loaded.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::load::errors::LoadError;
///
/// assert_eq!(LoadError::PackCorrupt.to_string(), "pack bytes do not decode");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    /// A rule's plan has a shape the lowering cannot express.
    #[error("rule {rule:?} not loaded: {shape:?}")]
    Unlowerable {
        /// The rule.
        rule: SelectedRuleOrdinal,
        /// What of its plan the lowering cannot express.
        shape: PlanShape,
    },
    /// A rule's program exceeds the lowering's budget: 4,096 instructions or 50 ms of lowering.
    #[error("rule {rule:?} not loaded: program over budget")]
    Budget {
        /// The rule.
        rule: SelectedRuleOrdinal,
    },
    /// The pack was written by another engine version.
    #[error("pack written by another engine version")]
    PackVersion,
    /// The pack was written for other rules.
    #[error("pack written for other rules")]
    PackDigest,
    /// The pack's bytes do not decode.
    #[error("pack bytes do not decode")]
    PackCorrupt,
}
