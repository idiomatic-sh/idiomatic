//! The rust domain's `targets` concept: the source file of every target Cargo finds in a package, by what the manifest declares and
//! by where the files are.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
