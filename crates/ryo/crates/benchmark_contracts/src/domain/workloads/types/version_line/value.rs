//! The existing value and its behaviour, owned together.
/// The exact line a pinned tool's version command prints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionLine(
    /// The line, without its line ending.
    pub Box<str>,
);

impl std::fmt::Display for VersionLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&super::super::Tool> for VersionLine {
    /// The line most pinned tools print: `<tool> <version>`.
    fn from(tool: &super::super::Tool) -> Self {
        Self(format!("{} {}", tool.tool, tool.version).into())
    }
}
