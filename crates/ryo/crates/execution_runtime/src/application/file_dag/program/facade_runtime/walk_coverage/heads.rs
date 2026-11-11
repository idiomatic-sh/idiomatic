//! Independent physical-head coverage over the authoritative relation catalog.
//! Expected shared heads, computed from the program and the catalog alone: a head of either
//! partition that selects a catalog relation is shared within its own partition.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
