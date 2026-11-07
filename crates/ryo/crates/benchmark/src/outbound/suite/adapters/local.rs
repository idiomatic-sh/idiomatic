//! Local boundary implementation.
pub(crate) mod adapter;
mod constants;
mod rule_cases;
mod corpus;
mod declarations;
mod observation;
mod process;
mod reference;
mod selection;
mod tool;
mod writer;

#[cfg(test)]
mod adapter_test;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;

mod discrepancy_artifacts;

mod case_observations;


mod input_metadata;
