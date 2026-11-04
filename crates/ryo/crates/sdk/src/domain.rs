//! The SDK's own values that more than one of its entries hands a caller or takes from one, one module per concept: each is the SDK's
//! one edge copy of its concept, and a resource module exposes it.

pub(crate) mod capacity;
pub(crate) mod cell;
pub(crate) mod file;
pub(crate) mod host;
pub(crate) mod language;
pub(crate) mod operator;
pub(crate) mod query;
pub(crate) mod relation;

pub(crate) mod observation;
