//! The `flow` concept: the control flow graphs a language's read works out of a file's functions, kept with the file's table for
//! the engine's readers of `flow-edge`, `flow-block`, `flow-member` and `flow-step`.
pub mod constants;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
