//! The `file_scan` operation (scope-engine2 2.7, track H): each file walked by its language, scanned by the new engine, and its
//! found rows kept as owned text and numbers for the store and the report. On the build branch it holds the comparison of the
//! `RYO_ENGINE` harness: the new engine's rows against the old engine's, rule by rule and file by file.

pub mod constants;
pub mod errors;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
