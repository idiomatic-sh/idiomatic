//! The types of the `reasons` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a language's tool or library said when a use case failed.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolMessage(pub Box<str>);

impl std::ops::Deref for ToolMessage {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for ToolMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
