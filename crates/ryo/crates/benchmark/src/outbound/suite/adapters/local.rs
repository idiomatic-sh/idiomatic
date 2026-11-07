//! Local boundary implementation.
pub(crate) mod adapter;
mod constants;
mod rule_cases;
mod corpus;
mod declarations;
mod evidence;
mod file_metadata;
mod process;
mod reference;
mod selection;
mod tool;
mod writer;

#[cfg(test)]
mod adapter_test;

#[cfg(any(test, feature = "fixtures"))]
mod fixtures;




