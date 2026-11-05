//! The existing value and its behaviour, owned together.
/// Stable benchmark project identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectName(
    /// Stable project name used in paths and observations.
    pub String,
);

impl From<&str> for ProjectName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ProjectName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for ProjectName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
