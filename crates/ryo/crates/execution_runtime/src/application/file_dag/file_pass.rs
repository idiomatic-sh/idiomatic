//! The file pass: one file's rules run through the operators' plan, the kernels reading the file's facts and tree in place.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
