#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this crate with no dependent: its parity test and the core's branch switch name its public items"
    )
)]

pub mod application;
pub mod domain;
pub mod inbound;
pub mod outbound;
