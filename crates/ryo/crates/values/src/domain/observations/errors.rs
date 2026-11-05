//! Refusal to interpret persisted readiness as a valid current publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
/// Persisted readiness is unusable for the current request.
pub enum ObservationReadinessError {
    /// Version, semantic layout, slot count or encoded states do not match the current request.
    #[error("observation readiness version, semantic layout, count or bits are invalid")]
    Invalid,
}
