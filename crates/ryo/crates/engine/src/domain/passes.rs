//! The `passes` concept: the load-time passes over a lowered pack, in their one order.

pub mod fusing;
pub mod hoisting;
pub mod memos;
pub mod numbering;
pub mod ordering;
pub mod prefixes;
pub mod service;

#[cfg(test)]
mod service_test;
