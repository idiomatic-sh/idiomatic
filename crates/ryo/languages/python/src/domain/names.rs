//! The python domain's `names` concept: the names a file writes, as rows hold them: dotted paths, the values of strings, annotations and assignments, and the reasons a name cannot be resolved statically.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
