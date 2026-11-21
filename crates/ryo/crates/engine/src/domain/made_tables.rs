//! The `made_tables` concept: the tables the engine makes for a file, each on its first read and once.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
