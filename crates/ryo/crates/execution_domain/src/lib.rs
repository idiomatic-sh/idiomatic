#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this domain with no dependent: the engine crates above it name its public items, and this crate's own unit tests do not"
    )
)]

pub mod application;
pub mod domain;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
