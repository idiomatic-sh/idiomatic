//! Persistent named-root readiness schema identity.
use crate::domain::records::types::ObservationReadinessVersion;
/// A missing/other version is unavailable; never infer Complete from missing rows.
pub(crate) const READINESS_VERSION: ObservationReadinessVersion = ObservationReadinessVersion(2);

/// Domain tag that opens the digest of the bound observations.
pub(crate) const BINDINGS_TAG: &[u8] = b"ryo-observation-bindings-v1";
/// Domain tag that opens the digest of the frontier readiness.
pub(crate) const FRONTIER_READINESS_TAG: &[u8] = b"ryo-observation-frontier-readiness-v1";
/// Domain tag that opens the digest of the demanded observation roots.
pub(crate) const DEMANDED_ROOTS_TAG: &[u8] = b"ryo-demanded-observation-roots-v1";
/// Domain tag that opens the digest of the named request layout.
pub(crate) const REQUEST_LAYOUT_TAG: &[u8] = b"ryo-named-request-layout-v2";
/// The reason a producer root left no readiness in a file generation.
pub(crate) const UNPUBLISHED_REASON: &str = "producer root did not publish in this file generation";
/// What a declaration count that does not fit in 64 bits says.
pub(crate) const COUNT_FITS_MESSAGE: &str = "a declaration count fits in u64";
