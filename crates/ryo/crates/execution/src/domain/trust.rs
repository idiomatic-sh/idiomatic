//! The `core` domain's `trust` concept: which reads of the relations a plan trusts the absence of rows of, and whether a relation read in
//! part could change what they make.

mod reads;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
