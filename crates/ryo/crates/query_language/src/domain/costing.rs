//! The `query_language` domain's `costing` concept: the cost class of a query as its text states it, read from its steps as
//! assembled and the sizes the catalog declares, the statement that sets it, and whether a pack's rules hold the ceiling and the
//! list its manifest declares. A compile answers the cost and refuses nothing for it.

mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod service_test;
