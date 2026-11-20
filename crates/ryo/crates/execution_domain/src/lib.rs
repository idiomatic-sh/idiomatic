#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this domain with no dependent: the engine crates above it name its public items, and this crate's own unit tests do not"
    )
)]

pub mod domain;
/// Spike (ryo-fmx3, bead ryo-r6ux): the plans as JSON for the engine bake-off.
pub mod spike_export;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
