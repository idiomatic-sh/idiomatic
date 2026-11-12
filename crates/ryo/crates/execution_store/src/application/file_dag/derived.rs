//! One file-local acquisition owner for engine-prepared immutable derived views.
pub mod owner;
pub mod prepare;
pub mod request;

pub mod prepared;

pub mod population;

pub mod admitted_views;
pub(crate) mod partition_views;
pub mod relation_owner;

pub mod constants;
