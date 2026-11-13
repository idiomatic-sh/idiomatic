#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

//! Shared benchmark data. This crate cannot issue audit or certification proofs.
//!
//! Audit and certification authority stay in the benchmark workflow.
//! ```compile_fail,E0432
//! use ryo_benchmark_contracts::domain::workloads::types::Audit;
//! ```
//! ```compile_fail,E0432
//! use ryo_benchmark_contracts::domain::workloads::types::Certificate;
//! ```
//! ```compile_fail,E0432
//! use ryo_benchmark_contracts::domain::measurements::types::BenchmarkSample;
//! ```
//! Transport encodings belong to the benchmark's outbound schemas.
//! ```compile_fail,E0277
//! use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
//! let _ = serde_json::to_string(&ExecutionMetadata::Complete);
//! ```
pub mod domain;

#[cfg(any(test, feature = "fixtures"))]
pub mod cases;
