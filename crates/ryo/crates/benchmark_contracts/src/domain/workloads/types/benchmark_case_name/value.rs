//! The existing value and its behaviour, owned together.
/// Stable human-readable identity of an independent case.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkCaseName(
    /// Unique human-readable case name within its rule.
    pub String,
);

impl From<&str> for BenchmarkCaseName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for BenchmarkCaseName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for BenchmarkCaseName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
