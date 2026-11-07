//! The existing value and its behaviour, owned together.
/// Expected executable SHA-256 digest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToolDigest(
    /// Expected SHA256 hexadecimal digest of executable bytes.
    pub String,
);

impl From<&str> for ToolDigest {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for ToolDigest {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for ToolDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
