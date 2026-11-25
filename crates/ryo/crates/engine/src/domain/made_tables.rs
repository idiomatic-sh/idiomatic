//! The `made_tables` concept: the tables the engine makes for a file, each on its first read and once.

pub(crate) mod built;
pub mod calling;
pub mod carrying;
pub(crate) mod columns;
pub mod constants;
pub mod fanning;
pub mod fills;
pub mod following;
pub mod indexing;
pub mod keyed;
pub mod making;
pub mod matching;
pub mod operands;
pub mod pipelines;
pub mod placing;
pub mod reading;
pub mod service;
pub mod sources;
pub mod staging;
pub mod types;
pub mod using;
pub(crate) mod visiting;
pub(crate) mod writing;

pub mod fixtures;

