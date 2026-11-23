//! What the `plan_codec` port asks of the outside: a plan's bytes, and the plan bytes hold.

use super::errors::PlanCodecError;
use ryo_values::domain::finish_plans::types::FinishPlan;

/// What every `plan_codec` adapter implements.
pub trait PlanCodecPort: Send + Sync {
    /// The plan's bytes.
    fn encoded(&self, plan: &FinishPlan) -> Vec<u8>;

    /// The plan as the JSON value its bytes would read back as, made in memory (no text written or parsed).
    fn valued(&self, plan: &FinishPlan) -> serde_json::Value;

    /// The plan the bytes hold.
    ///
    /// # Errors
    ///
    /// `Undecoded` when the bytes are not a plan.
    fn decoded(&self, bytes: &[u8]) -> Result<FinishPlan, PlanCodecError>;
}
