//! The application owns one execution and its admitted populations for each file
//! or project phase. Kernel code receives borrowed rows only after admission.
pub(crate) mod file_execution;
pub(crate) mod service;

#[cfg(test)]
mod walk_coverage;
