#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the CLI and language benchmark crates call public benchmark factories across crate boundaries; this crate's unit-test binary has no dependent"
    )
)]
pub mod application;
pub mod domain;
pub mod inbound;
pub mod outbound;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

#[cfg(test)]
mod cases_test;
