//! How an operator's rows grow with its inputs: the one declaration every operator of the physical plan makes, the one algebra
//! that composes a plan's declarations, the one check that refuses a node whose rows grow past its inputs', and the run budget
//! every making is held to.
pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
