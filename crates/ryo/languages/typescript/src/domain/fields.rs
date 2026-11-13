//! The typescript domain's `fields` concept: the role of every element under its parent, in the words of the TypeScript-ESTree
//! properties.

pub(crate) mod constants;
mod holding;
mod placing;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;

#[cfg(test)]
mod fixtures;
