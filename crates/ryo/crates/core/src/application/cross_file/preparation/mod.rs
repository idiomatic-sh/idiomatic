//! Declared project occurrences and their complete preparation before execution.
pub(crate) mod errors;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
