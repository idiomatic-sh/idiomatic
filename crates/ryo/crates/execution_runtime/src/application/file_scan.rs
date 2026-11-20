//! The `file_scan` operation (scope-engine2 2.7, track H): each file walked, scanned by the new engine and its rows handed to the
//! store and the report. On the build branch it holds the `RYO_ENGINE` harness: what the new engine's `load` accepts, and the
//! comparison of its rows with the old engine's, rule by rule and file by file.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
