//! The `needs` concept: whether a unit may have a row in the file, decided at run time from the formula the lowering
//! derived, once per file.

pub(crate) mod formulas;
pub(crate) mod masks;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
