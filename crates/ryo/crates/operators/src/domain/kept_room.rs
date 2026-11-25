//! The room a worker's scratch keeps from one file to the next: one cap and one rule for every kind of scratch it keeps across
//! files, so the room a big file grew is not kept for every file after it.
pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

