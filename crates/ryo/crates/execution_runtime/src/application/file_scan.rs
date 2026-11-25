//! The `file_scan` operation (scope-engine2 2.7, track H): each file walked by its language, scanned by the new engine, and its
//! found rows kept as owned text and numbers, each with where in the file it is, for the store and the report.

pub mod errors;
pub mod hosted;
pub(crate) mod keeping;
pub mod measuring;
pub mod service;
pub mod types;
pub mod warning;

