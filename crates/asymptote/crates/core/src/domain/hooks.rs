//! The `hooks` concept: the git hook scripts `asymptote init` writes, and how it knows a script is its own.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
