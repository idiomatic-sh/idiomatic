//! The `core` domain's `shared_walk` concept: the `select` steps of the call's rules over the elements of a file, merged at run time so
//! that the elements are visited once, each handed to every step that reads its kind, and a text is decided once for all of them.

pub(crate) mod service;
#[cfg(test)]
pub(crate) mod service_test;
pub(crate) mod types;
