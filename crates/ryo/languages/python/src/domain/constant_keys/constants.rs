//! Stable diagnostics for partial constant-key reads.

/// The parser supplied incomplete syntax or a required literal payload was unavailable.
pub(crate) const INCOMPLETE: &str =
    "constant keys require complete syntax and parser-owned literal payloads";
