//! The `core` domain's `linting` concept: what a check hands the lint call and how it reads what comes back: where the call
//! is rooted, the files it reads, the rules whose queries are refused, and the paths of its answers.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
