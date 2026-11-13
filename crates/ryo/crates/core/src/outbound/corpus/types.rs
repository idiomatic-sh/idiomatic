//! The `corpus` port's types: what the port hands back.

use ryo_values::domain::files::types::{CorpusEntry, CorpusInput};

/// Explicit inputs retain per-path discovery failures; available files carry only real observed metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorpusInventory(pub Vec<CorpusInput>);

/// The directories a walk never goes into: a name, at any depth (the dependency, environment and build directories of the linked
/// languages), or `./` and a path, the one directory at that path below the root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkippedDirectories(pub Vec<Box<str>>);

/// What the system says of a path: the file there, with its size and when it was last written; or that no file is there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathStat {
    /// A file is there.
    Found(CorpusEntry),
    /// Nothing is there, or it is not a file.
    Missing,
}
