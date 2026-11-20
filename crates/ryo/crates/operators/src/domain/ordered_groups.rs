//! Portable ordered-group semantics: the nearest rows of `as-of`, the ends of `group-min` and `group-max`, the places of `group-ordinal`.
pub mod service;

pub mod types;

#[cfg(test)]
mod service_test;
