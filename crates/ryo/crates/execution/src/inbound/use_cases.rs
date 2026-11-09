//! Execution requests and opaque run handles.
/// Compiles the rules of a request into plans, before any file is read.
pub mod compile_rules;
pub mod execution;
/// Pure request preflight, placement, coverage, and report metadata.
pub mod metadata;

/// Prepares a request's program for execution.
pub mod prepare;

/// Publishes a producer's names as portable values.
pub mod publication;
