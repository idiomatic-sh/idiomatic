//! The existing value and its behaviour, owned together.
/// Assessed capability identifier for a default disposition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapabilityName(
    /// Public capability identifier explaining an explicit deferral.
    pub String,
);

impl From<&str> for CapabilityName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for CapabilityName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for CapabilityName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
