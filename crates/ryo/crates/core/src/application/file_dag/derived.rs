//! One file-local acquisition owner for engine-prepared immutable derived views.
#[cfg(test)]
mod consumer_test;
pub(super) mod footprint;
pub(super) mod graph;
pub(crate) mod graph_cache;
pub(crate) mod indexes;
pub(super) mod owner;
#[cfg(test)]
pub(crate) mod prepare;
pub(super) mod request;
pub(crate) mod selection;
pub(crate) mod view;

pub(super) mod backend;

pub(super) mod ledger;
