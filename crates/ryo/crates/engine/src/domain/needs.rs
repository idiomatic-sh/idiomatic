//! The `needs` concept: whether a unit may have a row in the file, decided at run time from the formula the lowering
//! derived, once per file.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
