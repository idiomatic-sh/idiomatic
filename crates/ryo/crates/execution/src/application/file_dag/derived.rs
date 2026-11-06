//! One file-local acquisition owner for engine-prepared immutable derived views.
pub(super) mod owner;
pub(crate) mod prepare;
pub(super) mod request;

pub(super) mod ledger;

pub(in crate::application::file_dag) mod prepared;

pub(in crate::application::file_dag) mod population;

pub(in crate::application::file_dag) mod constants;
