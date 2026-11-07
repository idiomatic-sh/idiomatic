//! Execution requests and opaque run handles.
pub mod execution;
/// Pure request preflight, placement, coverage, and report metadata.
pub mod metadata;

/// Prepares a request's program for execution.
pub mod prepare;

/// Normalize portable publication data through private execution semantics.
pub mod publication;
