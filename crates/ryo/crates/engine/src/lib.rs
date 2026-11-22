#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this crate with no dependent: the host above it names its public items, and this crate's own unit tests do not"
    )
)]

pub mod domain;
pub mod inbound;
pub mod outbound;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
