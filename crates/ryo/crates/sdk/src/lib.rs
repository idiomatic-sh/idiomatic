#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

#![allow(
    clippy::pub_use,
    reason = "the SDK's one surface: each resource is named at the crate root"
)]

mod domain;
mod resources;

// The resources are the crate's own names, `ryo::query::validate`, as the command line says `ryo query validate`.
pub use resources::{languages, linter, operator, query, relation};

pub mod cases;
