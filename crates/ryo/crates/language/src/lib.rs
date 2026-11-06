#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this crate's public items with no dependent: every language crate, the core and the SDK build on its types and services, and its own unit tests do not"
    )
)]
#![cfg_attr(
    test,
    allow(
        clippy::unnecessary_wraps,
        reason = "the subject of a case returns a Result for the rows that refuse, whether or not a row of this part's cases does"
    )
)]

#[cfg(any(test, feature = "fixtures"))]
#[macro_use]
pub mod cases;

#[macro_use]
pub mod domain;
/// Engine-owned preparation and completion of one file generation.
pub mod application;
pub mod inbound;
