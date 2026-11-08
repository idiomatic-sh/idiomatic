//! The names the frontiers concept gives text.

/// Why a retained frontier is refused when its producer is unavailable.
pub(super) const PRODUCER_UNAVAILABLE: &str =
    "a producer feeding this retained frontier is unavailable";
/// Why a frontier is refused when every source partition was skipped.
pub(super) const SOURCES_SKIPPED: &str = "every source partition was explicitly skipped";
/// Why a stored frontier source is refused when its proof does not fit the program.
pub(super) const PROOF_MISMATCH: &str = "frontier source proof does not match the prepared program";
/// The span around the admission of one input.
pub(super) const INPUT_MEMORY_SPAN: &str = "cross_input_memory";
/// The phase named when a packed input is read.
pub(super) const PHASE_INPUT_PACKED: &str = "cross_input_packed";
/// The owner named for a packed input.
pub(super) const OWNER_PACKED_INPUT: &str = "packed_input";
/// The phase named when the compact inputs are prepared.
pub(super) const PHASE_COMPACT_PREPARED: &str = "compact_inputs_prepared";
/// The owner named for the frontier descriptors.
pub(super) const OWNER_FRONTIER_DESCRIPTORS: &str = "frontier_descriptors";
