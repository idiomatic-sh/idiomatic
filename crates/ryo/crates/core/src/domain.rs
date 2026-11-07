//! The `core` domain's rules, one module per concept.

pub(crate) mod admission;
#[cfg(any(test, feature = "fixtures"))]
pub(crate) mod allocation_verdict;
pub(crate) mod answering;
pub(crate) mod budget;
pub(crate) mod choosing;
#[cfg(any(test, feature = "fixtures"))]
pub mod corpus_copies;
pub(crate) mod crossing;
pub(crate) mod finish;
pub(crate) mod handing;
pub(crate) mod keeping;
pub(crate) mod listing;
pub(crate) mod ordering;

pub(crate) mod project;
pub(crate) mod reading;
pub(crate) mod recording;
pub(crate) mod relation_lookup;
pub(crate) mod render;
pub(crate) mod reporting;
pub(crate) mod repository;
pub(crate) mod rules;

pub(crate) mod storing;
pub(crate) mod value_cells;
pub(crate) mod visit;
pub(crate) mod workers;

// Demand-root join semantics now execute through the mandatory registered primitive.
