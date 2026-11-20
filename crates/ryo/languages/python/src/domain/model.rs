//! The python domain's `model` concept: the `global` and `nonlocal` names of every scope of a file, worked out once for its readers.

pub(crate) mod declarations;
pub(crate) mod definitions;
pub(crate) mod flow;
pub(crate) mod frames;
pub(crate) mod lexical;
pub(crate) mod names;
pub(crate) mod reaching;
pub(crate) mod resolution;
pub(crate) mod schedule;
pub(crate) mod service;
pub(crate) mod types;
pub(crate) mod view;

#[cfg(test)]
mod service_test;
