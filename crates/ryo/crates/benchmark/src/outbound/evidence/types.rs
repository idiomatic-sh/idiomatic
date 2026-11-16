//! Borrowed permission for the exact workflow-issued publication payload.
pub mod measured_publication;

/// How long the cases of one rule took to verify.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuleTiming {
    /// The time of every case of the rule together.
    pub total: std::time::Duration,
    /// The time of the rule's slowest case.
    pub slowest: std::time::Duration,
    /// The time the rule's query took to compile before its first case, none where each case compiles for itself.
    pub compile: std::time::Duration,
}
