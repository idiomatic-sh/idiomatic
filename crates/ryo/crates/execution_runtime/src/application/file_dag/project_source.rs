//! The project source: the relations the catalog declares `scope Project`, read once per phase from what core's project read
//! wrote, carried into the phase's symbols, each repeated row kept once.

pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
