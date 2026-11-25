//! What the files hand over to a rule's steps after the files: each entry of the rule's frontier read from the store, proven
//! against the files the rule read, and decoded once per phase.
pub(crate) mod constants;
pub mod errors;
pub(crate) mod service;
pub(crate) mod types;
