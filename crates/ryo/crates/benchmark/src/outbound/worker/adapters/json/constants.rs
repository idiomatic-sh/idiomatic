//! Process response framing and transport failure text.
/// Line framing appended after each complete JSON response.
pub(super) const RESPONSE_TERMINATOR: &[u8] = b"\n";
/// Refusal when a prior writer panicked while owning the response stream.
pub(super) const OUTPUT_LOCK_POISONED: &str = "worker output lock poisoned";
