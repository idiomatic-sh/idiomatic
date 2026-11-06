//! One file-local acquisition owner for engine-prepared immutable derived views.
pub(super) mod footprint;
pub(crate) mod graph_cache;
pub(crate) mod indexes;
pub(super) mod owner;
pub(crate) mod prepare;
pub(super) mod request;
pub(crate) mod selection;

pub(super) mod backend;

pub(super) mod ledger;

pub(in crate::application::file_dag) mod prepared;
