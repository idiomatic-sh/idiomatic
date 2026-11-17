//! How an operator's rows grow with its inputs: the one declaration every operator of the physical plan makes, the one algebra
//! that composes a plan's declarations, and the one check that refuses a node whose rows grow past its inputs'.
#[cfg(test)]
pub(crate) mod errors;
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
