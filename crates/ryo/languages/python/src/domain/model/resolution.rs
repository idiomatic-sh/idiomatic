//! The `resolution` part of the model: the one definition the language's static rule gives each name that is read, and whether
//! its origin is proven.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
