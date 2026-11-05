//! The `core` domain's `step_classes` concept: the `select` steps of the call's rules over the elements of a file, sorted into the
//! classes of the steps that select the same elements, so that the shared walk decides each class once.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
