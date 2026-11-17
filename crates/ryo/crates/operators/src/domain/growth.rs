//! How an operator's rows grow with its inputs: the one declaration every operator of the physical plan makes, and the one
//! algebra that composes a plan's declarations.
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
