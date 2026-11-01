//! Retirement of dead execution rows while preserving live file outputs.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
