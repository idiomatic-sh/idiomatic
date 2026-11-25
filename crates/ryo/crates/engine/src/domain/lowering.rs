//! The `lowering` concept: each selected rule's plan turned into its programs, or the plan shape the lowering cannot express named.

pub mod constants;
pub(crate) mod costing;
pub(crate) mod definitions;
pub(crate) mod emptiness;
pub mod errors;
pub(crate) mod flattening;
pub(crate) mod nodes;
pub(crate) mod observing;
pub(crate) mod placement;
pub(crate) mod planning;
pub(crate) mod proving;
pub mod service;
pub(crate) mod sharing;
pub(crate) mod tables;
pub mod types;

pub mod fixtures;

