//! The existing value and its behaviour, owned together.
/// Pinned reference tool identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolName(
    /// Reference tool name reported by its version command.
    pub String,
);

impl From<&str> for ToolName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ToolName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
