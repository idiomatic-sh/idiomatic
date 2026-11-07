//! The constants of the part: the text its code uses.

/// Why a project rule has no outcome when the engine answered fewer outcomes than tasks.
pub(crate) const MISSING_OUTCOME: &str = "project root outcome missing";

/// Why a project rule cannot be prepared when its frontier has no first bucket.
pub(crate) const MISSING_FRONTIER_BUCKET: &str = "missing frontier bucket";
