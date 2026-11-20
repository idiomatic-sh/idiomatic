// ignore: idiomatic.arch/file-length - the one instruction set the lowering writes, the packs encode and the executor runs, with the rule programs that hold it: a contract three tracks build against, so splitting it now would move names under all of them; the tracks that own it regroup it into its step items as they land
//! The types of the `programs` concept: a rule set's programs and the instruction set they are written in. Everything but
//! `Programs` itself is the crate's own: the lowering writes it, the packs encode it, the executor runs it.
#![expect(
    dead_code,
    reason = "the instruction set is the contract between the lowering (track L1), which writes it, the packs (track L2), which encode it, and the executor (track X), which runs it; until they land only the empty rule set exists, and the first of them to read a part removes this"
)]

use ryo_language::domain::facts::types::{BaseWords, CellWord};
use ryo_language::domain::node_table::types::FieldCode;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::NodeColumn;
use ryo_relations::domain::relations::types::{Column, KindBits, Relation};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// The programs of a rule set: opaque; built only by `load`, encoded by `packs::service`.
///
/// ```compile_fail,E0603
/// let forged = ryo_engine::domain::programs::types::Programs(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(accepted(&Programs::default()).is_empty());
/// ```
#[derive(Debug, Default)]
pub struct Programs(pub(crate) ProgramsEngine);

/// The private body of `Programs`.
#[derive(Debug, Default)]
pub(crate) struct ProgramsEngine {
    /// Each accepted rule's programs, in rule order.
    pub(crate) rules: Vec<RuleProgram>,
    /// Sub-programs more than one place runs (`Shared`), by number.
    pub(crate) shared: Vec<StepProgram>,
    /// The made tables the programs probe, by number.
    pub(crate) made: Vec<MadeTable>,
    /// The constant pool: every text a program compares with, as the run's base words.
    pub(crate) words: BaseWords,
    /// The regular expressions the programs match, by number, as their sources.
    pub(crate) regexes: Vec<StepRegex>,
    /// Every relation some program reads: what a language builds its model for (`RelationNeeds`).
    pub(crate) reads: Vec<Relation>,
}

/// One rule's programs: a union of branches, each with its anchor alternatives.
#[derive(Debug)]
pub(crate) struct RuleProgram {
    /// The rule.
    pub(crate) rule: SelectedRuleOrdinal,
    /// Its branches; a row any branch emits is the rule's.
    pub(crate) branches: Vec<Branch>,
    /// How many registers its programs use.
    pub(crate) registers: RegisterCount,
    /// What each column of the rows it emits holds: the lowering knows what it emits.
    pub(crate) columns: crate::domain::findings::types::RuleColumns,
}

/// One conjunctive branch of a rule: alternatives that answer the same rows from different anchors.
#[derive(Debug)]
pub(crate) struct Branch {
    /// The alternatives, cheapest first by the anchor statistics or the fixed cost model.
    pub(crate) alternatives: Vec<Alternative>,
}

/// One way to run a branch: its anchor and its program.
#[derive(Debug)]
pub(crate) struct Alternative {
    /// The kinds its first step iterates, when it starts with a kind bucket: what `KindsPresent` disables.
    pub(crate) anchor: Option<AnchorKinds>,
    /// The program.
    pub(crate) program: StepProgram,
    /// Its expected steps per file, from the statistics or the cost model.
    pub(crate) cost: StepCost,
}

/// A program: straight-line steps over registers, run once per anchor candidate.
#[derive(Debug, Default)]
pub(crate) struct StepProgram(pub(crate) Box<[ProgramStep]>);

/// The instruction set (scope-engine2 2.5): one variant per instruction, each item of `collections/steps` runs one. The lowering
/// writes these; the executor runs them; the packs encode them.
#[derive(Debug)]
pub(crate) enum ProgramStep {
    /// Anchor: each id of the kinds, in id order.
    EachKind { kinds: AnchorKinds, out: Register },
    /// Anchor: each row of a filled relation whose column holds the value.
    EachValue {
        relation: Relation,
        column: Column,
        value: StepOperand,
        out: Register,
    },
    /// Anchor: each row of a filled relation whose number column is between the bounds (inclusive).
    EachRange {
        relation: Relation,
        column: Column,
        low: StepOperand,
        high: StepOperand,
        out: Register,
    },
    /// Anchor: each row of a made table.
    EachRow {
        table: MadeTableId,
        out: RegisterList,
    },
    /// Anchor: each row of a filled relation.
    EachFile { relation: Relation, out: Register },
    /// Navigation: each child of a node, of one field when given.
    Children {
        parent: Register,
        field: Option<FieldCode>,
        out: Register,
    },
    /// Navigation: each element along an axis from a node.
    Axis {
        direction: AxisDirection,
        start: Register,
        out: Register,
    },
    /// Navigation: the test that one node is within another (descendant walks visit nodes only).
    Within {
        ancestor: Register,
        descendant: Register,
    },
    /// Navigation: each node reached through one or more edges, the edge a sub-program, seeded from `start`.
    Closure {
        edge: StepProgram,
        start: Register,
        out: Register,
    },
    /// Navigation: a column of a row, read in place from the node table or from the filled columns.
    Read {
        source: ReadSource,
        row: Register,
        out: Register,
    },
    /// Test of two operands; the program stops for the candidate when it fails.
    Test {
        test: TestOperation,
        left: StepOperand,
        right: StepOperand,
    },
    /// A per-row meaning of `operators` over the operands, writing its outputs.
    Call {
        meaning: CallMeaning,
        arguments: Box<[StepOperand]>,
        outputs: RegisterList,
    },
    /// The source form of a node in a syntax role, through `FactSource::render`.
    Render {
        node: Register,
        context: RenderContext,
        out: Register,
    },
    /// A sub-program that must have (or must not have) a row for the candidate; stops at its first row.
    Exists {
        polarity: Polarity,
        sub: StepProgram,
    },
    /// The candidate goes on only when the sub-program fails at once.
    FirstFails { sub: StepProgram },
    /// A union as one step: each sub-program writes the free registers; distinct bindings go on.
    Or {
        subs: Box<[StepProgram]>,
        outputs: RegisterList,
        memo: OrMemo,
    },
    /// Shared-prefix dispatch: each slot's program runs from here.
    Fork { slots: Box<[StepProgram]> },
    /// A segment answered once per key value and kept for the file.
    Hoist { key: Register, segment: StepProgram },
    /// Keeps the registers a hoisted segment answers.
    Capture { registers: RegisterList },
    /// A sub-program's row: its registers go to the step that runs it.
    Yield,
    /// Runs a shared sub-program in place.
    Shared { program: SharedId },
    /// A register takes a value.
    Let { from: StepOperand, out: Register },
    /// The rows of a made table whose key columns equal the key, on demand.
    Probe {
        table: MadeTableId,
        key: Box<[StepOperand]>,
        out: RegisterList,
    },
    /// The last row of a keyed made table whose order column is strictly before the bound.
    Latest {
        table: MadeTableId,
        key: Box<[StepOperand]>,
        bound: StepOperand,
        out: RegisterList,
    },
    /// Writes a row into a made table.
    Insert {
        table: MadeTableId,
        row: Box<[StepOperand]>,
    },
    /// Hands a row over: a finding of the rule, or a frontier row for the project pass.
    Emit {
        target: EmitTarget,
        columns: Box<[StepOperand]>,
    },
}

/// A register of a running program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Register(pub(crate) u16);

/// Registers, in an instruction's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RegisterList(pub(crate) Box<[Register]>);

/// How many registers a rule's programs use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RegisterCount(pub(crate) u16);

/// The kinds an anchor iterates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AnchorKinds(pub(crate) Box<[KindBits]>);

/// A program's expected steps per file.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StepCost(pub(crate) f64);

/// A made table, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MadeTableId(pub(crate) u32);

/// A shared sub-program, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SharedId(pub(crate) u32);

/// A regular expression of the constant pool, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RegexId(pub(crate) u32);

/// The source of a regular expression, compiled at load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StepRegex(pub(crate) Box<str>);

/// An operand of a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StepOperand {
    /// A register's value.
    Register(Register),
    /// A constant: a base word or a number's bits.
    Word(CellWord),
    /// A set of constants (`InC`).
    Words(Box<[CellWord]>),
    /// The scanned file.
    File,
}

/// Where `Read` reads a column from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadSource {
    /// A column of the node table, specialised at load from the relation's `NodeColumn`.
    Node(NodeColumn),
    /// A column of a filled relation, keyed by the row register.
    Filled(Relation, Column),
}

/// The comparisons a `Test` makes (with the load-time constant forms folded in).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TestOperation {
    /// Equal.
    Equal,
    /// Not equal.
    NotEqual,
    /// Less than.
    Less,
    /// Less than or equal.
    LessOrEqual,
    /// Greater than.
    Greater,
    /// Greater than or equal.
    GreaterOrEqual,
    /// In a set of constants.
    In,
    /// Not in a set of constants.
    NotIn,
    /// Matches a regular expression.
    Matches(RegexId),
    /// Matches a glob.
    Glob,
    /// Does not match a glob.
    NotGlob,
    /// Starts with.
    StartsWith,
    /// Ends with.
    EndsWith,
    /// `numeric>=`: numbers and canonical numeric text.
    NumericAtLeast,
    /// `natural<`: natural text order.
    NaturalLess,
    /// The right is the left plus one, both integers.
    Successor,
}

/// The axes of `Axis`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AxisDirection {
    /// The parent.
    Parent,
    /// Each child.
    Child,
    /// Each ancestor, nearest first.
    Ancestor,
    /// Each descendant node, in pre-order.
    Descendant,
    /// The next sibling.
    NextSibling,
    /// The previous sibling.
    PreviousSibling,
}

/// The per-row meanings of `operators` a `Call` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallMeaning {
    /// `span-between`.
    SpanBetween,
    /// `span-bounds`.
    SpanBounds,
    /// `source-matches`.
    SourceMatches,
    /// `capture`.
    Capture,
    /// `replace`.
    Replace,
    /// `words`.
    Words,
    /// `rank`.
    Rank,
    /// `concat`.
    Concat,
    /// `lower`.
    Lower,
    /// `fits-rendered`.
    FitsRendered,
}

/// Whether an `Exists` asks for a row or for none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Polarity {
    /// The sub-program must answer a row.
    Present,
    /// The sub-program must answer none.
    Absent,
}

/// Whether an `Or` reads no candidate register, so its answer is kept for the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OrMemo {
    /// Answered once per file.
    PerFile,
    /// Answered per candidate.
    PerCandidate,
}

/// Whose row an `Emit` hands over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmitTarget {
    /// A finding of the program's rule.
    Rule,
    /// A row of the rule's frontier entry, for the project pass.
    Frontier(InputNumber),
}

/// A table the programs make in the file and probe by key.
#[derive(Debug)]
pub(crate) struct MadeTable {
    /// How its rows are made.
    pub(crate) fill: MadeFill,
    /// How many key columns lead its rows.
    pub(crate) keys: ColumnCount,
    /// How many columns its rows have.
    pub(crate) width: ColumnCount,
}

/// How a made table's rows are made: by its programs' `Insert`s, or by a graph meaning over input programs.
#[derive(Debug)]
pub(crate) enum MadeFill {
    /// Rows the programs insert.
    Inserted(Box<[StepProgram]>),
    /// A group count or digest over an input.
    Grouped(Box<[StepProgram]>),
    /// An ordered match of two inputs.
    OrderedMatch(Box<[StepProgram]>, Box<[StepProgram]>),
    /// An ordered depth-first walk (roots, edges).
    OrderedDfs(Box<[StepProgram]>, Box<[StepProgram]>),
    /// A worklist fixpoint.
    Worklist(Box<[StepProgram]>),
    /// The cycles of a graph.
    Cycles(Box<[StepProgram]>),
}
