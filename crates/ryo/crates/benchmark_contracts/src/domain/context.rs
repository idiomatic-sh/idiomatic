//! Declared and observed benchmark context data.
pub mod constants;
pub mod types;

/// The content seal of a listed project context, shared by the context boundary and the case registrar.
pub mod service;

#[cfg(test)]
mod service_test;
