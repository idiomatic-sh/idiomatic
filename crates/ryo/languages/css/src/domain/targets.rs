//! The css domain's `targets` concept: what an `@import` or a `url()` points at, as a path of the run, and what each belongs to.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
