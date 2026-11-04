//! Persistent named-root readiness schema identity.
use super::types::ObservationReadinessVersion;
/// A missing/other version is unavailable; never infer Complete from missing rows.
pub const READINESS_VERSION: ObservationReadinessVersion = ObservationReadinessVersion(2);
