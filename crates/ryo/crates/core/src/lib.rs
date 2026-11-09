#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles the core's public items with no dependent: the command line crate and the SDK call its use cases, factories and fixtures, and the core's own unit tests do not"
    )
)]

pub mod application;
pub mod inbound;
pub mod outbound;

mod domain;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
