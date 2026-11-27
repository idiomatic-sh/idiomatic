//! The `context` concept: what a run gives its rules beside the catalog, lowered as steps. A context table is the
//! constant rows it holds (`rows`); a rule observation is the union of its producers' observed outputs, each kept to the
//! observation's columns.

pub(crate) mod service;
