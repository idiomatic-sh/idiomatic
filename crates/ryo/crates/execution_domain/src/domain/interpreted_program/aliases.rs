//! One part of the executable-program service: aliases of named outputs resolve into the same dependency graph once, each
//! region declares the partition of its relation inputs, and each region's lineage is found once, through the named observations
//! to the producer roots their members read.
pub(crate) mod service;
mod types;
