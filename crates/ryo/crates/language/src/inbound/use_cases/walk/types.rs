//! The values `walk` takes.

use super::errors::WalkError;
use crate::domain::facts::types::FileFacts;
use crate::domain::files::types::SourcePath;
use crate::domain::fillers::types::WalkVocabulary;
use crate::domain::node_table::types::NodeTable;
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, FileId, Relation};

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
/// use ryo_language::inbound::use_cases::walk::types::{WalkRequest, WalkSelection};
/// use ryo_relations::domain::relations::types::FileId;
///
/// let request = WalkRequest { path: SourcePath("a.walk"), file: FileId(0), source: ByteSlice(b"one"), selection: WalkSelection::Recognized };
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
    /// Whether the language reads the file by its path, or because the request names it to.
    pub selection: WalkSelection,
}

/// Why a language walks a file: it recognizes the path, or the request names the language to read every file it selects,
/// whatever the path (a file with no extension, a `.txt` file).
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::walk::types::WalkSelection;
///
/// assert_ne!(WalkSelection::Recognized, WalkSelection::Named);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkSelection {
    /// The language reads the files whose paths it recognizes.
    Recognized,
    /// The request names the language: it reads the file whatever the path.
    Named,
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

/// What the host asks of a linked language's walk, behind an interface alone, so that languages of different per-file views and
/// buffers are held side by side. Every `WalkUseCase` is one: the host never names a language's types.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FileFacts;
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_language::domain::node_table::types::NodeTable;
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_language::inbound::use_cases::walk::errors::WalkError;
/// use ryo_language::inbound::use_cases::walk::types::{FileVisit, RelationNeeds, WalkEngine, WalkRequest};
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// use ryo_relations::domain::relations::types::FileId;
///
/// struct Ignored;
/// impl FileVisit for Ignored {
///     fn visit(&mut self, _file: FileFacts<'_>) {}
/// }
/// let language: &dyn WalkEngine = &WalkUseCaseImpl::new();
/// let request = WalkRequest { selection: ryo_language::inbound::use_cases::walk::types::WalkSelection::Recognized, path: SourcePath("a.txt"), file: FileId(0), source: ByteSlice(b"") };
/// let (mut table, mut buffers) = (NodeTable::default(), language.buffers());
/// let walked = language.walk_file(request, RelationNeeds(&[]), &mut table, &mut *buffers, &mut Ignored);
/// assert_eq!(walked, Err(WalkError::Unsupported));
/// ```
pub trait WalkEngine: Send + Sync {
    /// How many kinds of node and token the language's trees have.
    fn kind_count(&self) -> Count;

    /// The names of the run (`Fillers::vocabulary`): the language's kinds, fields and fixed names, each by place. The host
    /// reads it once, before the first file.
    fn vocabulary(&self) -> WalkVocabulary;

    /// One worker's buffers of the language's own (`WalkUseCase::Scratch`), refilled for each file and never freed: the host
    /// holds them beside the worker's node table and names nothing inside.
    fn buffers(&self) -> Box<dyn std::any::Any + Send>;

    /// Walks one file into the worker's `table` and lends it to `visit` while the parse lives, as `WalkUseCase::walk` does.
    ///
    /// # Errors
    ///
    /// `Unsupported` when the language does not read the file; `Library` when the parser fails, or when `buffers` are another
    /// language's.
    fn walk_file(
        &self,
        request: WalkRequest<'_>,
        needs: RelationNeeds<'_>,
        table: &mut NodeTable,
        buffers: &mut (dyn std::any::Any + Send),
        visit: &mut dyn FileVisit,
    ) -> Result<(), WalkError>;
}
