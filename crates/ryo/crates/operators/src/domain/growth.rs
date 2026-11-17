//! How an operator's rows grow with its inputs: the one declaration every operator of the physical plan makes, the run budget
//! every making is held to, and the instances a growth test runs it on.
pub(crate) mod constants;
pub(crate) mod errors;
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
