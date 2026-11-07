//! Why a reference process failure is refused.

/// A refused reference process failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ProcessFailureError {
    /// Refuses: two observations share a raw capture.
    #[error("process observation groups must retain distinct raw captures")]
    GroupsShareCaptures,
    /// Refuses: the chosen executable has no reproducible group.
    #[error("missing reproducible exact-binary observation group")]
    NoReproducibleGroup,
    /// Refuses: an observation is of an executable the profile does not approve.
    #[error("process evidence has an unapproved exact binary")]
    UnapprovedExactBinary,
    /// Refuses: two observations share a raw artifact.
    #[error("process observations must retain separate raw artifacts")]
    ObservationsShareArtifacts,
    /// Refuses: the actual executable lacks two captures.
    #[error("process expectation needs two captures for the actual exact executable")]
    ActualBinaryNeedsTwoCaptures,
}
