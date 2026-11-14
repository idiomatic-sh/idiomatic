//! The rust domain's `model` concept: the indexes its readers build over a file the first time one asks, their capacity kept from
//! file to file.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
