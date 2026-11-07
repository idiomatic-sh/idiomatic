//! The rust domain's `module_tree` concept: the module tree of a crate, drawn from the facts of its files, and the paths a file writes resolved against it.

pub(crate) mod constants;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
