//! The types of the `files` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The path of a file, relative to the repository root, with `/` between directories.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilePath(pub Box<str>);

impl std::ops::Deref for FilePath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for FilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A path as a file has it in a repository, from the root of the run, borrowed from where the caller holds it: what a file is read as, and what a target it writes is resolved against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePath<'a>(pub &'a str);

impl std::ops::Deref for SourcePath<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl std::fmt::Display for SourcePath<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// The name of a file the language reads as a manifest: the last part of its path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManifestName(pub &'static str);

/// Which existing language operation owns an explicitly requested primary file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryFileRole {
    /// Source input; syntax may be accepted, rejected or unknown after an actual parse.
    Source,
    /// Project/configuration input; the project reader owns parsing and validation, not the source parser.
    Configuration,
}
