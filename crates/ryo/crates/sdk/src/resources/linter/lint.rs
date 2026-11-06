//! `linter lint`.

pub(crate) mod composition;
pub(crate) mod connections;
pub mod entry;
pub mod errors;
#[cfg(test)]
mod fixtures;
pub(crate) mod schemas;
pub mod types;

#[cfg(test)]
mod entry_test;

#[cfg(test)]
mod composition_test;
