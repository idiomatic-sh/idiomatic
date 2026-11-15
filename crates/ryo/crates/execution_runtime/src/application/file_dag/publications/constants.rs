//! The names the publications give text.

/// Why a producer's publication is refused.
pub(super) const PUBLICATION_INCOMPLETE: &str =
    "cross producer publication is unavailable or malformed";
/// Why a restored named output is refused when a frame of it belongs to a file its rule was syntax-rejected on.
pub(super) const RESTORED_FRAME_REJECTED: &str =
    "restored semantic frame belongs to a currently syntax-rejected producer partition";
