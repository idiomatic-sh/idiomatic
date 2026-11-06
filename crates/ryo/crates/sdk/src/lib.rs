#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles the SDK's public items with no dependent: programs that read or declare a language call its resources, and the SDK's own unit tests do not"
    )
)]
#![allow(
    clippy::pub_use,
    reason = "the SDK's one surface: each resource is named at the crate root"
)]

#[cfg(any(test, feature = "fixtures"))]
#[macro_use]
pub mod cases;

mod domain;
mod resources;

// The resources are the crate's own names, `ryo::query::validate`, as the command line says `ryo query validate`.
pub use resources::{languages, linter, operator, query, relation};
