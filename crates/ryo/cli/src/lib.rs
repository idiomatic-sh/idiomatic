#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles this library's public items with no dependent: the `ryo` binary and the benches call its entry and fixtures, and the library's own unit tests do not"
    )
)]

pub mod edge;
mod languages;
#[cfg(all(
    test,
    feature = "javascript",
    feature = "typescript",
    feature = "python",
    feature = "rust"
))]
mod languages_test;
pub mod outcome;
pub mod output;
pub mod resources;
pub mod timings;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
