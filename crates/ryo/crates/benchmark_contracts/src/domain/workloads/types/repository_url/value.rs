//! The existing value and its behaviour, owned together.
/// Repository remote used to obtain an immutable revision.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositoryUrl(
    /// Git remote used to fetch the pinned project.
    pub String,
);

impl From<&str> for RepositoryUrl {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for RepositoryUrl {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for RepositoryUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
