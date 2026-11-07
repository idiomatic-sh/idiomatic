//! The rust domain's `links` concept: how the source files of a project refer to one another, and the relations
//! that say it, one item each.

pub(crate) mod collections;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
