//! The sources a report reads.
use tastemaker_values::domain::files::types::FileText;
use tastemaker_values::domain::paths::types::RepoPath;

/// The sources a report reads, each read once, by its path.
#[derive(Debug, Default)]
pub(crate) struct SourceFiles(pub std::collections::BTreeMap<RepoPath, SourceFile>);

/// One source's text, and the byte each of its lines starts at, the empty last line after a newline included.
#[derive(Debug)]
pub(crate) struct SourceFile {
    /// The text.
    pub text: FileText,
    /// Where each line starts.
    pub starts: LineOffsets,
}

/// The byte each line of a source starts at, in order.
#[derive(Debug, Default)]
pub(crate) struct LineOffsets(pub Vec<u64>);
