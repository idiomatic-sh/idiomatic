//! The `language` domain's `completion` concept: the finishing of the table of a file once it is built.

pub mod service;

#[cfg(test)]
mod service_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
