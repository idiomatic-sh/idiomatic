//! The `core` domain's `table_reach` concept.

pub(crate) mod borrowed;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

#[cfg(test)]
mod ordered_test;
