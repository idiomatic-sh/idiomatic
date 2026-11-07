//! The `core` domain's `reporting` concept: the report a check makes of the lint call's answer: the rules' findings
//! rendered, the templates' and the imports' findings found, the ignores applied and every finding fingerprinted.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
