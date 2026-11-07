//! The go domain's `specs` concept: the parts of what a declaration lists: the names and the type of a constant or variable, the name
//! and the type of a type, and the name and the path of an import.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
