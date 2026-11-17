//! The executable program's `activation` concept: whether a node of the planned program can have a row or a row error in one
//! file, composed from its inputs' states and its operator's declared requirement, from triggers read once per file.
#[cfg(test)]
mod fixtures;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
