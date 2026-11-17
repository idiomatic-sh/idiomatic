//! The `judgement` concept: what a run's violations mean against the accepted records. Two violations are the same
//! exactly when their keys are equal: no cascade, no fuzzy matching.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
