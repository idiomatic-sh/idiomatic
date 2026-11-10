#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]
#![cfg_attr(
    test,
    expect(
        dead_code_pub_in_binary,
        reason = "the unit-test build is the one binary that compiles these values with no dependent: the core, the query language and the SDK name them, and this crate's own unit tests do not"
    )
)]
#![cfg_attr(
    test,
    allow(
        clippy::unnecessary_wraps,
        reason = "the subject of a case returns a Result for the rows that refuse, whether or not a row of this part's cases does"
    )
)]

pub mod domain;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;

#[cfg(test)]
mod cases_test;
