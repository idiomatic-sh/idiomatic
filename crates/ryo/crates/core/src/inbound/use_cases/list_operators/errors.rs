//! The `list_operators` use case's failures.

/// Why `list_operators` failed: it cannot, the operators are the domain's own, but every use case names its error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ListOperatorsError {}
