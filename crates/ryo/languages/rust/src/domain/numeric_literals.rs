//! The rust domain's `numeric_literals` concept: what a numeric literal node of a file's tree holds of a number, as the syntax adapter
//! stored it: its kind (an integer or a binary64 float), its value, and the type suffix written after its digits.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
