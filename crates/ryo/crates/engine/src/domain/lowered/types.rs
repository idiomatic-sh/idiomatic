//! The types of the `lowered` concept: what the lowering writes and the load-time passes rewrite: the pack's definitions and made tables once each, then each rule's own program. Crate-private; dropped once encoded.

use crate::domain::findings::types::RuleColumns;
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::types::{
    AnchorKinds, ColumnList, ColumnMask, ConstantText, DefinitionId, MadeColumn, MadeTableId,
    RegisterCount, RegisterList, StepProgram, StepRegex,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Column, Relation};
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// A rule set as lowered: the pack's definitions and tables once each, then each rule's own program.
#[derive(Debug, Default)]
pub(crate) struct LoweredPack {
    /// Every definition some rule reaches, by number; named by its merged plan node.
    pub(crate) definitions: Vec<PackDefinition>,
    /// Every made table some program reads, by number; one per (fill, merged plan node).
    pub(crate) tables: Vec<MadeTable>,
    /// Each accepted rule's own program, in rule order.
    pub(crate) rules: Vec<LoweredRule>,
    /// How many registers the pack's programs name.
    pub(crate) registers: RegisterCount,
    /// The regular expressions the programs match, by number.
    pub(crate) regexes: Vec<StepRegex>,
    /// The constant texts, by number: base words at load.
    pub(crate) texts: Vec<ConstantText>,
    /// Every relation some program reads.
    pub(crate) reads: Vec<Relation>,
}

/// A sub-plan lowered once for the pack: its rows are a function of the file alone.
#[derive(Debug)]
pub(crate) struct PackDefinition {
    /// The merged plan node it is the rows of.
    pub(crate) node: NodeDigest,
    /// One register per column: a caller's bound cells arrive in the formals of the bound columns.
    pub(crate) formals: RegisterList,
    /// What must hold of the file for it to have a row.
    pub(crate) emptiness: Emptiness,
    /// One pattern per (bound columns, collected columns) some use asks.
    pub(crate) patterns: Vec<DefinitionPattern>,
}

/// How a definition answers one kind of use.
#[derive(Debug)]
pub(crate) struct DefinitionPattern {
    /// Which columns arrive bound.
    pub(crate) bound: ColumnMask,
    /// Which columns a row gives back.
    pub(crate) collected: ColumnMask,
    /// How it is evaluated.
    pub(crate) form: PatternForm,
}

/// The two ways a pattern is evaluated.
#[derive(Debug)]
pub(crate) enum PatternForm {
    /// Per distinct cells of the bound columns, the answer kept for the file: the program reads the bound formals and ends each row in a `Yield` of the collected formals, ascending.
    Call(StepProgram),
    /// From the definition's table, made once per file and read by an index on the bound columns.
    Table(MadeTableId),
}

/// What a file must hold for a node to have a row: decided once per file, before anything of the node runs.
#[derive(Debug, Default)]
pub(crate) enum Emptiness {
    /// Nothing is known before it runs.
    #[default]
    Unknown,
    /// A node of one of the kinds.
    Kind(AnchorKinds),
    /// A row of a filled relation, with the value in the column when given.
    Row(Relation, Option<RowValue>),
    /// A row of a definition: its own formula.
    Definition(DefinitionId),
    /// A row of a made table: its own formula.
    Table(MadeTableId),
    /// Every part holds; none: always.
    All(Box<[Emptiness]>),
    /// Some part holds; none: never.
    Any(Box<[Emptiness]>),
}

/// A value a row must hold in a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowValue {
    /// The column.
    pub(crate) column: Column,
    /// The value.
    pub(crate) value: CellWord,
}

/// One rule's own program.
#[derive(Debug)]
pub(crate) struct LoweredRule {
    /// The rule.
    pub(crate) rule: SelectedRuleOrdinal,
    /// The merged plan node of the step it emits: what its unit is named by.
    pub(crate) node: NodeDigest,
    /// Its branches; a row any branch emits is the rule's.
    pub(crate) branches: Vec<Branch>,
    /// What each column of the rows it emits holds.
    pub(crate) columns: RuleColumns,
}

/// One conjunctive branch of a rule.
#[derive(Debug)]
pub(crate) struct Branch {
    /// What the file must hold for the branch to run at all.
    pub(crate) emptiness: Emptiness,
    /// The program; its first step is its anchor.
    pub(crate) program: StepProgram,
}

/// A table made in the file once and read by key.
#[derive(Debug)]
pub(crate) struct MadeTable {
    /// The merged plan node it is the rows of.
    pub(crate) node: NodeDigest,
    /// How its rows are made.
    pub(crate) fill: MadeFill,
    /// How many columns its rows have.
    pub(crate) width: ColumnCount,
    /// The column sets it is read by, each an index.
    pub(crate) indexes: Vec<ColumnList>,
    /// What must hold of the file for it to have a row.
    pub(crate) emptiness: Emptiness,
}

/// How a made table's rows are made: the four kinds of table-making node, and a definition's own rows.
#[derive(Debug)]
pub(crate) enum MadeFill {
    /// A definition's rows with nothing bound: column i is the definition's column i.
    Rows(Box<[Pipeline]>, RepeatedRows),
    /// A group table over one input.
    Group(GroupFill, FillInput),
    /// An ordered table.
    Ordered(OrderedFill),
    /// A graph table over its inputs, in the operation's input order.
    Graph(GraphFill, Box<[FillInput]>),
    /// Rows written in the query or bound to a parameter: made once for the run.
    Constant(ConstantRows),
}

/// The group tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroupFill {
    /// `group-count`: the keys, then the count.
    Count { distinct: Option<MadeColumn> },
    /// `group-digest`: the keys, then the digest.
    Digest { over: MadeColumn },
    /// `group-min`, `group-max`: the input's columns.
    Ends(GroupEnd),
    /// `group-ordinal`: the input's columns of its distinct rows, then the place.
    Places,
}

/// The ordered tables.
#[derive(Debug)]
pub(crate) enum OrderedFill {
    /// The right rows of an `as-of`, by (keys, order): the input's columns; read by `Latest`.
    Sorted(FillInput),
    /// `ordered-match` of a left and a right input: the operation's output columns.
    Match(FillInput, FillInput),
}

/// The graph tables: each has the operation's output columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphFill {
    /// `follow`: the keys, the source, the destination.
    FixedPoints,
    /// `ordered-dfs`.
    OrderedDfs,
    /// `worklist`.
    Worklist,
    /// `cycles`.
    Cycles,
}

/// The pipelines whose rows are a fill's input, with the key and order columns of those rows.
#[derive(Debug)]
pub(crate) struct FillInput {
    /// The pipelines; a row is `Pipeline.row`.
    pub(crate) pipelines: Box<[Pipeline]>,
    /// The group key columns of a row.
    pub(crate) keys: ColumnList,
    /// The order column of a row, when the fill orders.
    pub(crate) order: Option<MadeColumn>,
}

/// Rows known at load.
#[derive(Debug, Default)]
pub(crate) struct ConstantRows(pub(crate) Vec<Box<[CellWord]>>);
