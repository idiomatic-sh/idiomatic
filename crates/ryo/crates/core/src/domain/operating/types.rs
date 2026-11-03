//! The types of the `operating` concept: what an operator reads, writes and fails with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::RowCount;

use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_model::types::Rows;
use crate::domain::table_model::types::Table;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::relations::types::CellKind;
use ryo_language::domain::relations::types::Value;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::stored_facts::types::ArenaBytes;

use crate::domain::table_groups::types::ValueHash;

/// An application-owned, file-scoped semantic demand port. Operators borrow it;
/// they never own a provider session or its cache.
pub(crate) trait SemanticLookup: std::fmt::Debug {
    /// Resolve demanded keys in order, reusing bounded file-scoped answers.
    fn render_batch(&self, requests: &[RenderRequest], source: &[u8]) -> Result<Vec<RenderOutcome>, OperatingError>;
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

/// What one step keeps from file to file, which no other step touches: its own room to work in, the rows of its inputs it holds to
/// read in place (the side a join looks up, what a group counts), and the rows it computes when it is not one that streams.
#[derive(Debug, Default)]
pub(crate) struct Room {
    /// What a `select` has decided about the names of the file.
    pub memo: Vec<Memo>,
    /// The room the step reads and sorts rows in.
    pub scratch: OperatorScratch,
    /// The rows of each input, when the step holds them.
    pub held: [Table; 3],
    /// The rows the step computes whole, to be passed on.
    pub out: Table,
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

/// The columns a walk from the starts is made by: the one that holds the start, and the one of the domain that holds the entities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WalkColumns {
    /// The column of the start.
    pub id: ColumnIndex,
    /// The column of the domain.
    pub domain: ColumnIndex,
}

/// What a walk is filtered by and what it answers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WalkShape<'o> {
    /// The comparisons the domain's elements must satisfy.
    pub comparisons: &'o [Comparison],
    /// The columns the rows answered are made of; empty for the start and the entity found.
    pub output: &'o [JoinColumn],
}

/// What a join of rows with a tree is filtered by and what it answers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeShape<'a> {
    /// The comparisons the elements must satisfy.
    pub comparisons: &'a [Comparison],
    /// The columns of the elements the comparisons read, as the join names them.
    pub view: &'a [ColumnIndex],
    /// The columns the rows answered are made of.
    pub output: &'a [JoinColumn],
}

/// How many starts a walk will have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StartCount(pub usize);

/// How many elements a tree has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementCount(pub usize);

/// How the rows of an input of a step come to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Supply {
    /// Made as they are asked for: not at hand to read in place.
    Streamed,
    /// At hand, to read in place.
    AtHand,
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

/// Whether a walk for the nearest ancestor keeps the answers it found for the elements it passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Remembering {
    /// The answers are kept, by element.
    Kept,
    /// Each start is answered by itself.
    Forgotten,
}

/// A column the rows of a tree lack, by its place among the columns the comparisons read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MissingColumn(pub usize);

/// A value met in a group, which a count of distinct values counts once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GroupValue {
    /// The group.
    pub group: GroupNumber,
    /// The value.
    pub value: Value,
}

/// The hash of a value met in a group, for the digest of the group's values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GroupHash {
    /// The group.
    pub group: GroupNumber,
    /// The hash of the value.
    pub hash: ValueHash,
}

/// A group of rows, by its place among the groups in the order they first appear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct GroupNumber(pub u32);

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
    pub regexes: usize,
    pub globs: usize,
    pub owners_initialized: usize,
    pub row_capacity: usize,
    pub other_capacity: usize,
    pub index_rows: usize,
    pub index_cells: usize,
    pub index_capacity: usize,
    pub closure_partition_rows: usize,
    pub closure_partition_capacity: usize,
    pub closure_visited: usize,
    pub dfs_expanded: usize,
    pub dfs_examined: usize,
    pub dfs_adjacency_capacity: usize,
    pub dfs_adjacency_len: usize,
    pub dfs_frames_len: usize,
    pub dfs_visited_len: usize,
    pub traversal_queue_len: usize,
    pub local_resources_released: bool,
}

/// Optional test telemetry is dormant until a real execution publishes observations.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct ExecutionObservationSlot(Option<Box<ExecutionObservation>>);
#[cfg(test)]
impl std::ops::Deref for ExecutionObservationSlot {
    type Target = ExecutionObservation;
    fn deref(&self) -> &Self::Target {
        const ZERO: ExecutionObservation = ExecutionObservation {
            regexes: 0,
            globs: 0,
            owners_initialized: 0,
            row_capacity: 0,
            other_capacity: 0,
            index_rows: 0,
            index_cells: 0,
            index_capacity: 0,
            closure_partition_rows: 0,
            closure_partition_capacity: 0,
            closure_visited: 0,
            dfs_expanded: 0,
            dfs_examined: 0,
            dfs_adjacency_capacity: 0,
            dfs_adjacency_len: 0,
            dfs_frames_len: 0,
            dfs_visited_len: 0,
            traversal_queue_len: 0,
            local_resources_released: false,
        };
        self.0.as_deref().unwrap_or(&ZERO)
    }
}
#[cfg(test)]
impl std::ops::DerefMut for ExecutionObservationSlot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_or_insert_with(Default::default)
    }
}
