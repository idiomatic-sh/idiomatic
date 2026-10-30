//! The types of the `scaffolding` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The name of the file an entry places at an instance: its pattern with the captures given and the extension in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedName(pub String);

/// A capture a placed file's name still holds, which nothing gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unfilled(pub String);

/// A param a template declares that nothing gave a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingParam(pub String);

/// Whether a placed name is one file or still a pattern of many.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Naming {
    /// A path: one file.
    Path,
    /// A pattern: a `*` or `?` is left, so the name stands for many files.
    Pattern,
}

/// The packages a workspace's files hold, and the files left that are its own: a local package's files are its package's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalSplit {
    /// The packages whose directory holds a file of the workspace.
    pub packages: Vec<crate::domain::definitions::types::LocalPackage>,
    /// The files no local package holds.
    pub governed: Vec<crate::domain::paths::types::RepoPath>,
}

/// Whether an entry's files are optional: an instance needs none of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Optionality {
    /// The instance may have none of its files.
    Optional,
    /// The instance needs at least one.
    Required,
}

/// Whether a template says what a first writing of its file starts it with: its content reads the scaffold flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstWriting {
    /// The content reads the flag, so a first writing has something to start the file with.
    Said,
    /// The content does not read it, or there is no template.
    Unsaid,
}

/// The captures the optional files of a spec introduce: a file whose path names one of them waits for that file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Waits(pub std::collections::BTreeSet<crate::domain::captures::types::CaptureName>);

/// Whether a placed file waits for an optional file that introduces a capture its name still holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waiting {
    /// It names a capture an optional file introduces: a collection's items need the collection.
    Waits,
    /// It waits for nothing.
    Ready,
}

/// Whether an entry places the module file that declares an instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Declaring {
    /// The entry is required and its file, placed under its base with the instance's captures, is the module file.
    Declares,
    /// It places some other file, or none of its files is needed.
    Other,
}

/// The names an entry reads from what `--set` gives: the captures its patterns hold and the params it declares or passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reads(pub std::collections::BTreeSet<crate::domain::captures::types::CaptureName>);

/// Where an instance's files are written, and the values its patterns and templates take.
#[derive(Debug)]
pub struct Placing<'a> {
    /// The instance's directory.
    pub directory: &'a crate::domain::paths::types::RepoPath,
    /// The values its patterns and templates take.
    pub values: &'a crate::domain::captures::types::Captures,
}

/// A prospective file and the values its entry needs before placement fills its context.
#[derive(Clone, Debug)]
pub struct Draft {
    /// The instance's directory.
    pub directory: crate::domain::paths::types::RepoPath,
    /// The values its patterns and templates take.
    pub values: crate::domain::captures::types::Captures,
    /// The template entry that places it.
    pub entry: crate::domain::entries::types::TemplateEntry,
}

/// Files planned by this invocation; nothing is written until their conditions and contents settle.
#[derive(Debug, Default)]
pub struct Staging {
    /// The prospective files, by path.
    pub drafts: std::collections::BTreeMap<crate::domain::paths::types::RepoPath, Draft>,
    /// The order they were planned in.
    pub order: Vec<crate::domain::paths::types::RepoPath>,
    /// The directories they need.
    pub directories: std::collections::BTreeSet<crate::domain::paths::types::RepoPath>,
    /// Their rendered texts, by path.
    pub texts: std::collections::BTreeMap<
        crate::domain::paths::types::RepoPath,
        crate::domain::files::types::FileText,
    >,
}
