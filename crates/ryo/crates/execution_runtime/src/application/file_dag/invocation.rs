//! Invocation-owned acquisition at the contract's demand boundary; no primitive cache policy.
mod binding;
mod constants;
mod groups;
pub(crate) mod index;
#[cfg(test)]
mod observation;
mod prepared_use;
pub(crate) mod service;
mod views;
