//! Borrowed semantic adaptation; only acquisition can construct and retain its payload.
mod semantic_rows;
pub(in crate::application::file_dag) mod service;
pub mod storage;
pub(in crate::application::file_dag) mod types;
