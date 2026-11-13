//! The existing value and its behaviour, owned together.
/// Full immutable Git commit identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectRevision(
    /// Immutable Git commit identifier.
    pub String,
);

impl From<&str> for ProjectRevision {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ProjectRevision {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for ProjectRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
