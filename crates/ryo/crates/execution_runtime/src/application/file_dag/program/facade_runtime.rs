//! The application owns one execution and its admitted populations for each file
//! or project phase. Kernel code receives borrowed rows only after admission.
pub(crate) mod file_execution;
pub(crate) mod project_execution;
pub(crate) mod service;
mod source_admission;
pub(crate) mod types;

#[cfg(test)]
mod compiled_parity;
#[cfg(test)]
mod walk_coverage;
