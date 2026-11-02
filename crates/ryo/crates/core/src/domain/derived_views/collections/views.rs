//! Every current relational index/view family implements the same mandatory interface.
pub(crate) mod interface;
pub(crate) mod registration;

pub(crate) mod items;

mod key;

mod predicate;

mod membership;
mod native;

pub(crate) mod graph;

#[cfg(test)]
mod registration_test;
