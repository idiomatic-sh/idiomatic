//! What the `plan_codec` port asks of the outside: a plan's bytes, and the plan bytes hold.

use super::errors::PlanCodecError;
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep};

/// What every `plan_codec` adapter implements.
pub trait PlanCodecPort: Send + Sync {
    /// The plan's bytes.
    fn encoded(&self, plan: &FinishPlan) -> Vec<u8>;

    /// Write a single plan node to a diagnostic stream.
    ///
    /// # Errors
    ///
    /// Returns the output error if the writer cannot accept the record.
    fn write_step(&self, step: &FinishStep, writer: &mut dyn std::io::Write) -> std::io::Result<()>;

    /// The plan the bytes hold.
    ///
    /// # Errors
    ///
    /// `Undecoded` when the bytes are not a plan.
    fn decoded(&self, bytes: &[u8]) -> Result<FinishPlan, PlanCodecError>;
}
