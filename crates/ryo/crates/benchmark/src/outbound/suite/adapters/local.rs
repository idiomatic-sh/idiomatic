//! Local boundary implementation.
pub(crate) mod adapter;
mod constants;
mod cases;
mod corpus;
mod declarations;
mod observation;
mod tool;

#[cfg(test)]
mod adapter_test;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;

mod discrepancy_artifacts;

mod case_observations;

#[cfg(test)]
mod process_schema_test;

mod input_metadata;
