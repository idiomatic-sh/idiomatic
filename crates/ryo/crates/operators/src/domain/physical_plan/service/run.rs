//! The runner: a node's kernels over one file's rows, its rows held until its last reader, and its fault after them.

pub(crate) mod counting;
pub(crate) mod minted;
pub(crate) mod pipeline;
pub(crate) mod reading;
pub(crate) mod service;

pub(crate) mod types;

pub(super) mod entity_queries;
