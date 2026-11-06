//! A production-source insertion and its precise Rust diagnostic contract.
#[derive(Clone, Copy, Debug)]
pub(super) enum Compilation {
    Library,
    LibraryTests,
    WantedMutation,
}
pub(super) struct Probe {
    pub(super) compilation: Compilation,
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
