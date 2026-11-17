//! The `captures` concept: the `{name}`s a tool's inputs write, what each took on disk, and the tool each set of values
//! makes, its id, command lines and inputs with the values written in.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
