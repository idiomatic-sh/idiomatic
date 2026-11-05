//! The existing value and its behaviour, owned together.
/// Exact reference release identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolVersion(
    /// Exact required reference version.
    pub String,
);

impl From<&str> for ToolVersion {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ToolVersion {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
