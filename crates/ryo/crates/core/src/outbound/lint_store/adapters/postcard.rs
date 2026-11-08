//! The store port, implemented with a file written as postcard writes its tables.

pub(crate) mod adapter;
#[cfg(test)]
mod buckets;
pub(crate) mod connection;
mod constants;
mod files;
mod findings;
mod proofs;
mod rest;
mod schemas;
mod strings;
mod tables;
mod unknowns;
pub(crate) mod types;
pub(crate) mod writer;

#[cfg(test)]
mod adapter_test;
