//! The warning of a rule that passes its work allowance in a file: the engine's notice in the run's words, naming the rule,
//! the file and the statement, with what is wrong and how to fix it. Nothing here stops a rule: its findings are complete.

pub(crate) mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
