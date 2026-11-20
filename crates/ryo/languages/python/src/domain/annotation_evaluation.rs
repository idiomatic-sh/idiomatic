//! Annotation evaluation timing: for each element directly inside a function-signature or variable annotation, that annotation and
//! when the language evaluates it.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
