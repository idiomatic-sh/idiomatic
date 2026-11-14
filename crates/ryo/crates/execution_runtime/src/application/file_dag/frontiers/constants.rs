//! The names the frontiers concept gives text.

/// Why a retained frontier is refused when its producer is unavailable.
pub(super) const PRODUCER_UNAVAILABLE: &str =
    "a producer feeding this retained frontier is unavailable";
/// Why a frontier is refused when every source partition was skipped.
pub(super) const SOURCES_SKIPPED: &str = "every source partition was explicitly skipped";
/// Why a stored frontier source is refused when its proof does not fit the program.
pub(super) const PROOF_MISMATCH: &str = "frontier source proof does not match the prepared program";
/// Why a restored named output is refused when a frame of it belongs to a file its rule was syntax-rejected on.
pub(super) const RESTORED_FRAME_REJECTED: &str =
    "restored semantic frame belongs to a currently syntax-rejected producer partition";
