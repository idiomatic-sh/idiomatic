//! The `file_analysis` application operation.
pub mod constants;
pub mod errors;
pub(crate) mod engine_source;
pub(crate) mod evaluation;
#[cfg(test)]
mod fixtures;
pub(crate) mod gating;
pub(crate) mod language_source;
pub(crate) mod outcome;
pub(crate) mod parsing;
pub(crate) mod project;
pub(crate) mod replaying;
pub(crate) mod semantic;
pub mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod streaming;
pub mod types;
