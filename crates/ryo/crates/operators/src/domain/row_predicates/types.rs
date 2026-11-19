//! Inputs, instructions and per-file state for ordered predicate evaluation.
use crate::domain::contract::types::PredicateOutcome;
use rustc_hash::FxHashMap;
use ryo_relations::domain::relations::types::Bits;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The ordinal of a distinct literal spelling in the prepared program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiteralId(pub usize);

/// The ordinal of a predicate in the original conjunction.
#[derive(Clone, Copy, Debug)]
pub struct TestIndex(pub usize);

/// The ordinal of an index on a shared input and column.
#[derive(Clone, Copy, Debug)]
pub struct PredicatePartitionId(pub usize);

/// The required outcome of an equality comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Equality {
    /// The operands must match.
    Equal,
    /// The operands must differ.
    Different,
}

/// How many matching rows the caller observes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchDemand {
    /// The caller observes the entire ordered bag.
    Rows,
    /// The caller only needs its first witness.
    First,
}

/// Whether the consumer can share ordered positions with siblings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Read in physical order.
    Scan,
    /// Permit an index after a second consumer reads the same input.
    Shared,
}

/// A test bound to the packet's row representation, or delegated to its original comparator.
#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    /// Compare a symbol word to a nonempty canonical spelling.
    Word(ColumnIndex, LiteralId, Equality),
    /// The two possible results of testing a flag's spelling.
    Flag(ColumnIndex, PredicateOutcome, PredicateOutcome),
    /// Compare two row words whose comparison is defined by their representations.
    Same(ColumnIndex, ColumnIndex, Equality),
    /// Ask the owning evaluator about this authored predicate.
    Original(TestIndex),
}

/// Ownership of the selection's input at the packet boundary.
#[derive(Clone, Copy, Debug, Default)]
pub enum SelectionInput {
    /// The provider retains its own dispatch and index policy.
    #[default]
    Provider,
    /// Ordered positions may be shared over this immutable step.
    Derived(StepId),
}

/// A conjunction and the immutable derived input whose ordered positions it may share.
#[derive(Debug, Default)]
pub struct PredicateSelection {
    /// A provider with its own dispatch policy has no shared input here.
    pub source: SelectionInput,
    /// Tests in authored order, with the first failure ending evaluation.
    pub instructions: Vec<Instruction>,
}

/// The values siblings request in one immutable input column.
#[derive(Debug)]
pub struct Partition {
    /// The column naming the partition key.
    pub column: ColumnIndex,
    /// Candidate keys, each named once.
    pub literals: Vec<LiteralId>,
}

/// One selection's shared source-column index and its requested value.
#[derive(Clone, Copy, Debug)]
pub struct Subscription {
    /// A partition in the prepared program.
    pub partition: PredicatePartitionId,
    /// The value demanded by this selection.
    pub literal: LiteralId,
}

/// Immutable program shared across files.
#[derive(Debug)]
pub struct Program {
    /// One conjunction per physical step.
    pub selections: Vec<PredicateSelection>,
    /// Shared indexes chosen from equality predicates with common parents.
    pub partitions: Vec<Partition>,
    /// The optional ordered index of each selection.
    pub subscriptions: Vec<Option<Subscription>>,
    /// Indexes to retire when the corresponding source is released.
    pub releases: Vec<Vec<PredicatePartitionId>>,
}

/// File-local state of a source that can have multiple readers.
#[derive(Debug, Default)]
pub enum Positions {
    /// The source has no readers yet.
    #[default]
    Unread,
    /// The first reader scanned the source.
    ReadOnce,
    /// Positions of requested keys in their original row order.
    Indexed(FxHashMap<Bits, Vec<RowIndex>>),
}

/// How many times a selection or index operation occurred.
#[derive(Clone, Copy, Debug, Default)]
pub struct Calls(pub usize);

/// Optional observations accumulated by the caller's file state.
#[derive(Debug, Default)]
pub struct Counts {
    /// Evaluated selections.
    pub selections: Calls,
    /// Rows in their source tables.
    pub input_positions: RowCount,
    /// Rows that reached the conjunction.
    pub inspected: RowCount,
    /// Shared indexes built.
    pub builds: Calls,
    /// Rows read while building indexes.
    pub build_positions: RowCount,
    /// Selections using a built index.
    pub indexed_reads: Calls,
    /// Selections whose required spelling was absent.
    pub absent: Calls,
}

/// A single file's bindings and lazy indexes over its immutable inputs.
#[derive(Debug)]
pub struct File<'a> {
    /// Prepared program borrowed for this file.
    pub plan: &'a Program,
    /// Each literal's canonical word, or its absence in this file.
    pub words: Vec<Option<Bits>>,
    /// The state of each shared index.
    pub positions: Vec<Positions>,
    /// The index policy for this execution.
    pub access: Access,
    /// Observations of rows and index operations.
    pub counts: Counts,
}
