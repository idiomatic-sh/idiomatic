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
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::stored_facts::types::ArenaBytes;

use crate::domain::table_groups::types::ValueHash;

/// The families of comparison a `select` tests with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Family {
    /// Equal and not equal.
    Equality,
    /// Less, greater, and their or-equal.
    Ordering,
    /// Numbers written as text.
    FloatText,
    /// In and not in.
    Membership,
    /// Glob matches.
    Glob,
    /// Starts with and ends with.
    Affix,
}

/// What an operator reads besides its inputs and may write: what the host bound, the names and
/// bytes of the file the rows are of, and the paths of the files. An operator may add the names it
/// makes to the file's names.
#[derive(Debug)]
pub(crate) struct OperatorContext<'a> {
    /// What each parameter is bound to, by the parameter's place.
    pub bindings: &'a [ParameterBinding],
    /// The names of the file.
    pub symbols: &'a mut Symbols,
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
    /// The columns of the input the step is reading one row at a time.
    pub kinds: Vec<CellKind>,
    /// What a group has counted, group by group.
    pub counts: GroupCounts,
    /// The values already counted for a group, by the group.
    pub pairs: hashbrown::HashSet<GroupValue>,
    /// The hash of each value of a group, by the group.
    pub hashed: Vec<GroupHash>,
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

/// What a step reads its inputs through: it can ask for the rows of an input one at a time, or hold them whole.
pub(crate) trait Feed<'f> {
    /// What the columns of the input hold.
    ///
    /// # Errors
    ///
    /// `NotRead` when the input is a relation nothing provides.
    fn kinds(&self, which: InputPlace) -> Result<&[CellKind], OperatingError>;

    /// Whether the rows of the input are made as they are asked for (`Streamed`), so that they are not at hand to read in place.
    fn streams(&self, which: InputPlace) -> Supply;

    /// Passes the rows of the input to `emit`, one by one, as they are made.
    ///
    /// # Errors
    ///
    /// Whatever the steps that make them or `emit` refuse.
    fn stream(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError>;

    /// Notes that a step looked up `rows` rows in the input, which it holds as a tree and does not read whole: the rows it found, and
    /// not the rows the tree has, are what the input was fed.
    fn found(&mut self, which: InputPlace, rows: RowCount);

    /// The rows of the input, whole, to read in place: a relation as it is; the rows of a step, in `into`.
    ///
    /// # Errors
    ///
    /// Whatever the steps that make them refuse.
    fn hold<'t>(
        &mut self,
        which: InputPlace,
        context: &mut OperatorContext<'_>,
        into: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't;
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

/// The room an operator computes in, over rows it holds whole.
#[derive(Debug)]
pub(crate) struct HeldRoom<'a> {
    /// What the comparisons decided about the names.
    pub memo: &'a mut Vec<Memo>,
    /// The room the operators read and sort rows in.
    pub scratch: &'a mut OperatorScratch,
    /// The rows computed.
    pub out: &'a mut Table,
}

/// The room an operator computes in, over rows it holds whole, with the table its starts are held in.
#[derive(Debug)]
pub(crate) struct StreamRoom<'a> {
    /// The room to compute in.
    pub held: HeldRoom<'a>,
    /// The table the starts are held in.
    pub from: &'a mut Table,
}

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

/// What a group has counted, by the group.
#[derive(Debug, Default)]
pub(crate) struct GroupCounts(pub Vec<usize>);

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
