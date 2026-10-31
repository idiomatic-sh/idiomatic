//! Fresh-process resource isolation.
#![cfg_attr(
    not(test),
    expect(
        clippy::cfg_not_test,
        reason = "these three production modules are exercised by the explicit collector integration target with the real Cargo-built helper"
    )
)]
#[cfg(not(test))]
pub(crate) mod adapter;
#[cfg(not(test))]
mod constants;
mod schemas;
#[cfg(not(test))]
pub(crate) mod settings;
