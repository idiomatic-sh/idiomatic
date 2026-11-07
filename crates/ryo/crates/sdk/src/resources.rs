//! What a user of the product calls: `<resource>::<entry>`, one module per resource.

#![allow(
    clippy::pub_use,
    reason = "the SDK surface re-exports what a user is aware of, only here (ED4): a `pub use` elsewhere in the crate is still caught"
)]

pub mod languages;
pub mod linter;
pub mod operator;
pub mod query;
pub mod relation;
