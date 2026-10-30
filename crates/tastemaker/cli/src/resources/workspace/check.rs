//! `check`.

pub mod command;
pub(crate) mod composition;
pub(crate) mod connections;
pub(crate) mod schemas;

#[cfg(test)]
mod command_test;

#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod fixtures;
