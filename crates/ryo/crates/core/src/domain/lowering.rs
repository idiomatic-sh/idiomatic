//! The `core` domain's `lowering` concept: which steps of a plan read only the tree of a file, and what each is when it runs over the
//! positions of the elements.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
