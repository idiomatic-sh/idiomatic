//! The types of the `lowered` concept: what the lowering writes and the load-time passes rewrite: the pack's definitions and made tables once each, then each rule's own program. Crate-private; dropped once encoded.

use crate::domain::code::records::types::EdgeColumns;
use crate::domain::code::types::ShapeChoice;
use crate::domain::findings::types::RuleColumns;
use crate::domain::steps::stages::types::{Pipeline, PlacedStage};
use crate::domain::steps::types::{
    AnchorKinds, ColumnList, ColumnMask, ConstantText, DefinitionId, MadeColumn, MadeTableId,
    PlacedStep, RegisterCount, RegisterList, StepProgram, StepRegex,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Column, Count, Relation};
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// A rule set as lowered: the pack's definitions and tables once each, then each rule's own program.
#[derive(Debug, Default)]
pub(crate) struct LoweredPack {
    /// Every definition some rule reaches, by number; named by its merged plan node, and one per node: two uses of one node,
    /// in one rule or in two, name one definition. "N definitions" in a load's answer is how many of these the pack holds,
    /// counted by node: the definitions lowered.
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

/// A sub-plan lowered once for the pack: its rows are a function of the file alone. A plan step that is flattened into what
/// reads it is no definition; a made table that is not a definition's own rows is none either.
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
    /// Whether its form was the shape rule's to decide or is a sample's: the lowering marks each flip candidate `Open`,
    /// whichever form it gave it, and the encoding writes the mark into the pattern's unit.
    pub(crate) shape: ShapeChoice,
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
    /// The merged plan node it is the rows of. A `Rows` table is a definition's own and bears that definition's node.
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
    /// `ordered-match` of a left and a right input, in that order.
    ///
    /// The inputs. Each input's rows are its pipelines' rows. `keys` are the columns of such a row the two sides are
    /// equated on, the left's and the right's lists of one length and compared place by place; `order` is the column of
    /// its order, a number, and both inputs have one.
    ///
    /// The pairs, as the operators crate's meaning makes them today. Within the rows of one key, the left rows are taken
    /// in ascending order; each takes the earliest right row of that key whose order is greater than its own and that no
    /// left row before it took. A left row with no such right row gives nothing, and a right row is in at most one pair.
    /// So of one key with the orders 1, 2, 3 on each side, the pairs are (1, 2) and (2, 3): the operators crate's case
    /// `ordered_match_pairs_first_in_first_out`.
    ///
    /// The rows. One row for each pair: the left row's cells in their order, then the right row's cells in theirs. The
    /// table is as wide as the two rows together, and column `c` of the right row is its column `left width + c`
    /// (`lowered::service::match_column`). The operation's own `output` is no part of the fill: the lowering reads each
    /// output column from this row by place. Within a key the rows stand in ascending order of the left row's order; the
    /// order of the keys among themselves is the fill's and no reader may lean on it.
    ///
    /// Ties. None is ordered: two rows of one input with one key and one order are the fault `DuplicateOrder` of
    /// today's meaning, and a left row and a right row of one order do not pair.
    ///
    /// The index. A reader reads the table as it reads a group table: by any of its `indexes`, each a list of this row's
    /// columns and none of them ordered, or every row with no key. `Latest` does not read it.
    Match(FillInput, FillInput),
}

/// The graph tables. A variant holds the columns of its inputs' rows that its kernel is told and `FillInput` has no field
/// for: the record the encoding writes and the executor reads (`code::records`). What `FillInput.keys` and `.order` are
/// for each input is said with the variant. The columns of the table's row, in order, are
/// `lowered::graphs::service::graph_row`; a reader finds a column's place with `graph_column`, and the encoding writes no
/// graph table that is not the width `graph_width` answers. A reader reads a graph table as it reads a group table: by
/// any of its `indexes`, each a list of this row's columns and none of them ordered, or every row with no key; `Latest`
/// does not read it. The order of a graph table's rows is the fill's, and no reader may lean on it.
///
/// What the budget counts of a graph fill is `HeaviestWork::Graph`: `read`, the rows of its inputs and every visit its
/// kernel makes of an edge after the first; `made`, the rows it writes. Nothing stops a graph fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphFill {
    /// `follow`: each source with every fixed point it reaches.
    ///
    /// The input. One, the edges: its rows are its pipelines' rows, and the columns held here are the source and the
    /// destination of such a row, cells of one kind. Its `keys` are the columns that part the edges into graphs that
    /// never meet, none for one graph of the file; its `order` is none.
    ///
    /// The row. The keys' cells in the order of `keys`, then the source, then the fixed point: `Key(0)..`, `Source`,
    /// `Destination`, two columns more than the keys. This is the order of the operation's own answer.
    ///
    /// The rows, as `ryo_operators::domain::follow::service::fixed_points` makes them of one key's edges. A fixed point
    /// is an entity with an edge to itself, and it answers itself. One row for each pair of a source and a fixed point
    /// its edges lead to, each pair once: a repeated edge adds nothing, and a source with several targets has a row
    /// for every fixed point it reaches. A walk that runs out of edges gives no row, and neither does a ring of more
    /// than one edge that reaches no fixed point. So of one key 7 with the edges (1, 2), (2, 3), (3, 3) the rows are
    /// `[7, 1, 3]`, `[7, 2, 3]`, `[7, 3, 3]`: the operators crate's case
    /// `each_source_answers_its_fixed_point_and_a_self_pointer_answers_itself`.
    ///
    /// The cost. Class `Group`: the work never crosses a key. Within one it is the key's edges, sorted once, and one
    /// walk back from each fixed point: the edges when each source has one target, and at most the edges times the
    /// fixed points when sources have several. The rows are then every pair of a source and a fixed point it reaches,
    /// which are the plan's own rows; the walks' visits are counted as `read`, so a key whose walks pass the rule's
    /// allowance is a notice that names this table.
    FixedPoints(EdgeColumns),
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

/// A unit of a pack as its walks meet it: what is numbered and counted as one program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WalkedUnit {
    /// A definition's pattern in call form: the definition, and the pattern's place among its patterns.
    Pattern(DefinitionId, Count),
    /// A rule's own program, all its branches.
    Rule(SelectedRuleOrdinal),
    /// A made table's fill, all its pipelines.
    Table(MadeTableId),
}

/// What the walk that lends a pack lends: a step of a program or a stage of a pipeline, with its plan point, to write.
#[derive(Debug)]
pub(crate) enum LentPlaced<'a> {
    /// A step of a definition's call form or of a rule's branch.
    Step(&'a mut PlacedStep),
    /// A stage of a pipeline of a table's fill.
    Stage(&'a mut PlacedStage),
}
