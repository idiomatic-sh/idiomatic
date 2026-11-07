//! The types of the `operating` concept: what an operator reads, writes and fails with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;

use crate::domain::table_model::types::Rows;
use ryo_values::domain::query_bindings::types::ParameterBinding;

use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::stored_facts::types::ArenaBytes;

/// An application-owned, file-scoped semantic demand port. Operators borrow it;
/// they never own a provider session or its cache.
pub trait SemanticLookup: std::fmt::Debug {
    /// Resolve demanded keys in order; the current application deduplicates each batch.
    fn render_batch(
        &self,
        requests: &[RenderRequest],
        source: &[u8],
    ) -> Result<Vec<RenderOutcome>, OperatingError>;
}

/// The families of comparison a `select` tests with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Family {
    /// Equal and not equal.
    Equality,
    /// Less, greater, and their or-equal.
    Ordering,
    /// Numbers written as text.
    FloatText,
    /// Exact canonical numeric text and whole numbers.
    Numeric,
    /// In and not in.
    Membership,
    /// Glob matches.
    Glob,
    /// Starts with and ends with.
    Affix,
    /// Regular expression searches.
    Regex,
}

/// What an operator reads besides its inputs and may write: what the host bound, the names and
/// bytes of the file the rows are of, and the paths of the files. An operator may add only derived
/// names; language fact readers alone own source-name interning.
#[derive(Debug)]
pub(crate) struct OperatorContext<'a> {
    /// Execution phase supplied by the owner, independent of available text arenas.
    pub phase: AfterTheFiles,
    /// The current file's application-owned semantic session, when available.
    pub semantic: Option<&'a dyn SemanticLookup>,
    /// Finite authored literal matchers owned by this worker's prepared request.
    pub prepared_matchers:
        Option<&'a crate::domain::matching::types::prepared::value::PreparedMatchers>,
    /// Finite prepared literal-kind metadata for the current exact vocabulary.
    pub prepared_kinds: Option<&'a crate::domain::kind_sets::types::PreparedVocabulary>,

    /// What each parameter is bound to, by the parameter's place.
    pub bindings: &'a [ParameterBinding],
    /// Readable file names and a derived-only query interning capability.
    pub symbols: QuerySymbols<'a>,
    /// The bytes of the file.
    pub source: SourceView<'a>,
    /// The path of each file of the run, by file number.
    pub paths: &'a [FilePath],
    /// The text each worker made, by worker number, when the steps run after the files were read.
    pub arenas: &'a [ArenaBytes],
    /// What a `select` has already decided about a name, for each comparison and each name of the
    /// file: kept between steps and files so it allocates once.
    pub memo: &'a mut Vec<Memo>,
    /// The room the operators read and sort rows in.
    pub scratch: &'a mut OperatorScratch,
}

/// What a comparison decided about a name of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Memo {
    /// Not decided yet.
    Unknown,
    /// The comparison fails for the name.
    Fails,
    /// The comparison holds for the name.
    Holds,
}

/// Where the rows of a step go as they are made, one at a time. The context is the one of the step that makes them; the
/// step that takes them makes its own from it.
pub(crate) trait Emit {
    /// Takes one row.
    ///
    /// # Errors
    ///
    /// Whatever the step that takes it refuses.
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError>;
}

/// How many names a file has: the length of a comparison's run of the memo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NameCount(pub usize);

/// A comparison of a `select`, by its place among the select's comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComparisonNumber(pub usize);

/// One of the inputs of a step, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputPlace(pub usize);

/// What a join of rows with a tree is filtered by and what it answers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeShape<'a> {
    /// The comparisons the elements must satisfy.
    pub comparisons: &'a [PlanComparison],
    /// The columns of the elements the comparisons read, as the join names them.
    pub view: &'a [ColumnIndex],
    /// The columns the rows answered are made of.
    pub output: &'a [JoinColumn],
}

/// Whether a comparison asks of the kind of an element something fixed, which the sets of kinds decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KindNaming {
    /// It names a kind.
    Names,
    /// It does not.
    Silent,
}

/// Whether the kinds of a filter leave few enough elements to look at only those.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KindNarrowing {
    /// The elements to look at are listed.
    Narrowed,
    /// The whole tree is looked at.
    Whole,
}

/// A column the rows of a tree lack, by its place among the columns the comparisons read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MissingColumn(pub usize);

/// Whether the steps run before the files were read, or after: after, the text of the workers is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AfterTheFiles {
    /// While the files are read.
    Before,
    /// After the files were read.
    After,
}
/// Test-only observations captured from real engine-owned resources before release.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ExecutionObservation {
    /// Successful real native-position evaluations.
    pub native_positions: usize,
    /// Calls to the real registered operator dispatch.
    pub registered_calls: usize,
    pub regexes: usize,
    pub globs: usize,
    pub owners_initialized: usize,
    pub row_capacity: usize,
    pub other_capacity: usize,
    pub index_rows: usize,
    /// Actual borrowed representation behind the acquired equality resource.
    pub index_representation:
        Option<crate::domain::table_model::collections::representations::types::RepresentationKind>,
    pub index_cells: usize,
    pub index_capacity: usize,
    pub closure_partition_rows: usize,
    pub closure_partition_capacity: usize,
    pub closure_visited: usize,
    pub dfs_expanded: usize,
    pub dfs_examined: usize,
    pub dfs_adjacency_len: usize,
    pub dfs_frames_len: usize,
    pub dfs_visited_len: usize,
    pub traversal_queue_len: usize,
    /// Set only after a real weak file-owner handle is observed dead at file finish.
    pub file_owner_released: bool,
}

use ryo_values::domain::rows::types::{RowCount, Table};
