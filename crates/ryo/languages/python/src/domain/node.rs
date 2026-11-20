//! The python domain's `node` concept: the parser's own tree of one file, read in place by the fillers of the engine's relations.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
