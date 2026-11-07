//! The typescript domain's `catalogs` concept: dependency specs, and the workspace catalogs that give a `catalog:` spec its version.

pub(crate) mod constants;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
