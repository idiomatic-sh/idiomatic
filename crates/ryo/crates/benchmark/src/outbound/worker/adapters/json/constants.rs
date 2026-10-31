//! Process response framing and transport failure text.
pub(super) const RESPONSE_TERMINATOR: &[u8] = b"\n";
pub(super) const OUTPUT_LOCK_POISONED: &str = "worker output lock poisoned";
