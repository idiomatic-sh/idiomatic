//! Stable vocabulary of a corpus comparison.
/// Refusal of a request whose selected rules are not exactly the configured rules.
pub(super) const SELECTION_DIFFERS: &str =
    "parity requires the request to select exactly the configured rules";
/// Refusal of a run with differences the accepted differences do not cover.
pub(super) const UNACCEPTED: &str = "unaccepted parity differences";
