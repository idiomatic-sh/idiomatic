//! Invocation-owned acquisition at the contract's demand boundary; no primitive cache policy.
mod binding;
mod constants;
mod groups;
pub(crate) mod index;
#[cfg(test)]
mod observation;
mod prepared_use;
mod row_metadata;
pub(crate) mod service;
pub(crate) mod types;
mod views;
