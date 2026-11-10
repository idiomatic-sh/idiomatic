//! Local boundary implementation.
mod accepted;
pub(crate) mod adapter;
mod constants;
mod corpus;
mod declarations;
mod evidence;
mod file_metadata;
mod process;
mod reference;
mod rule_cases;
mod selection;
mod tool;
mod writer;

#[cfg(test)]
mod adapter_test;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;
