//! The `made_tables` concept: the tables the engine makes for a file, each on its first read and once.

pub mod calling;
pub mod carrying;
pub mod constants;
pub mod fanning;
pub mod fills;
pub mod indexing;
pub mod keyed;
pub mod making;
pub mod matching;
pub mod operands;
pub mod pipelines;
pub mod reading;
pub mod service;
pub mod sources;
pub mod staging;
pub mod types;
pub mod using;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
