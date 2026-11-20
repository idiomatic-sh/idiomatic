//! The values `walk` takes.

use crate::domain::facts::types::FileFacts;
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{FileId, Relation};

/// Where the engine takes the file the language lends.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FileFacts;
/// use ryo_language::inbound::use_cases::walk::types::FileVisit;
///
/// struct Ignored;
/// impl FileVisit for Ignored {
///     fn visit(&mut self, _file: FileFacts<'_>) {}
/// }
/// let _visit = Ignored;
/// ```
pub trait FileVisit {
    /// Runs the engine on the lent file.
    fn visit(&mut self, file: FileFacts<'_>);
}

/// The file to walk: its path, its number and its bytes.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_language::inbound::use_cases::walk::types::WalkRequest;
/// use ryo_relations::domain::relations::types::FileId;
///
/// let request = WalkRequest { path: SourcePath("a.walk"), file: FileId(0), source: ByteSlice(b"one") };
/// assert_eq!(request.file, FileId(0));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct WalkRequest<'a> {
    /// The file's path, relative to the repository root.
    pub path: SourcePath<'a>,
    /// The number the run gave the file: the `file` cell of its rows.
    pub file: FileId,
    /// The bytes.
    pub source: ByteSlice<'a>,
}

/// The relations some loaded program can read in this file: what the language builds its model for.
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::walk::types::RelationNeeds;
///
/// assert!(RelationNeeds(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RelationNeeds<'a>(pub &'a [Relation]);
