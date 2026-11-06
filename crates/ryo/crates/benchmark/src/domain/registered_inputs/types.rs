//! Data owned by the registered input identities.

/// The words of a refusal that a library answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RefusalText(pub String);
