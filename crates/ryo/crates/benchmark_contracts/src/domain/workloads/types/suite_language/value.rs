//! The existing value and its behaviour, owned together.
/// Language identifier for a shared corpus.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SuiteLanguage(
    /// Language identifier shared by the project corpus.
    pub String,
);

impl From<&str> for SuiteLanguage {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for SuiteLanguage {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for SuiteLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
