//! The `core` domain's `kind_check` concept: the words a rule writes for a kind of node or token are checked against the kinds the linked
//! languages have, so a name that no language has is refused before anything is read.

pub(crate) mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
