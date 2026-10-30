//! The `core` domain's `layout` concept: how a plan is laid out for the worker that runs it, which steps keep their rows and what
//! columns each answers.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
