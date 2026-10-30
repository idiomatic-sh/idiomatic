//! The `describe` use case's failures.

/// Why `describe` failed: it cannot, the grammar is the syntax adapter's own, but every use case names its error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, Copy)]
pub enum DescribeError {}
