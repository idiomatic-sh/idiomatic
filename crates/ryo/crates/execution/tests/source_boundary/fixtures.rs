//! A production-source insertion and its precise Rust diagnostic contract.
pub(super) struct Probe {
    pub(super) owner: &'static str,
    pub(super) source: String,
    pub(super) expected: Expectation,
}
pub(super) enum Expectation {
    Accept,
    Refuse {
        code: &'static str,
        first: &'static str,
        second: &'static str,
    },
}
