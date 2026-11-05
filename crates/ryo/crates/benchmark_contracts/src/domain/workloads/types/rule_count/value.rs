//! The existing value and its behaviour, owned together.
/// Number of rules in a pinned coverage inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleCount(
    /// Number of rule identities in the audited inventory.
    pub usize,
);

impl std::fmt::Display for RuleCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
