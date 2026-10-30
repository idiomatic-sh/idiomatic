//! The go domain's `fact_notes` concept: what the parse leaves of a file for the project to read, the import paths it writes.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
