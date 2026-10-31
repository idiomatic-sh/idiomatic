//! Fresh-process resource isolation.
#[expect(
    clippy::cfg_not_test,
    reason = "the collector integration target tests this boundary with the real Cargo-built helper executable"
)]
#[cfg(not(test))]
pub(crate) mod adapter;
#[expect(
    clippy::cfg_not_test,
    reason = "the collector integration target tests this boundary with the real Cargo-built helper executable"
)]
#[cfg(not(test))]
mod constants;
mod schemas;
#[expect(
    clippy::cfg_not_test,
    reason = "the collector integration target tests this boundary with the real Cargo-built helper executable"
)]
#[cfg(not(test))]
pub(crate) mod settings;
