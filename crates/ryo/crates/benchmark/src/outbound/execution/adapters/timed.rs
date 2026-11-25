//! Fresh-process resource isolation.
#![cfg_attr(
    not(test),
    expect(
        clippy::cfg_not_test,
        reason = "these three production modules are exercised by the explicit collector integration target with the real Cargo-built helper"
    )
)]
pub(crate) mod adapter;
mod constants;
mod schemas;
pub(crate) mod settings;
