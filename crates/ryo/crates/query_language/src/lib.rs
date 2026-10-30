#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this crate's public items with no dependent: the core calls its use cases and fixtures, and its own unit tests do not"
    )
)]

pub mod inbound;
pub mod outbound;

mod domain;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
