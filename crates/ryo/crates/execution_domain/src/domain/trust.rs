//! The `core` domain's `trust` concept: which reads of the relations a plan trusts the absence of rows of, and whether a relation read in
//! part could change what they make.

mod reads;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
