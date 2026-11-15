//! The `RuleOrdinal` value.

/// A rule of a program, by its place in rule order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleOrdinal(pub u32);

impl RuleOrdinal {
    /// The next rule's place.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}
