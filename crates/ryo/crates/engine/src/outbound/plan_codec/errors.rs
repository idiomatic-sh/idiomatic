//! The `plan_codec` port's failures.

/// Why bytes are not a plan.
///
/// # Examples
///
/// ```
/// use ryo_engine::outbound::plan_codec::errors::PlanCodecError;
///
/// let error = PlanCodecError::Undecoded("expected value at line 1 column 1".into());
/// assert!(error.to_string().starts_with("not a finish plan"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanCodecError {
    /// The bytes do not read as a plan.
    #[error("not a finish plan: {0}")]
    Undecoded(String),
}
