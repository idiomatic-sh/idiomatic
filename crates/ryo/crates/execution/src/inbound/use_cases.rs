//! The use cases of the execution crate: rule compilation, metadata and publication.
/// Compiles the rules of a request into plans, before any file is read.
pub mod compile_rules;
/// Pure request preflight, placement, coverage, and report metadata.
pub mod metadata;

/// Publishes a producer's names as portable values.
pub mod publication;
