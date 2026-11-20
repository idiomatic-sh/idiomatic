//! The types of the `code` concept: a rule set's programs as the executor runs them and the pack stores them: fixed-width words in one arena, operand lists and constants in side pools, every memo site, index and table numbered. Crate-private.

use crate::domain::findings::types::RuleColumns;
use crate::domain::steps::types::ColumnMask;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// One instruction: sixteen bytes, read in place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CodeWord {
    /// What it does.
    pub(crate) op: Op,
    /// The variant of the operation: polarity, direction, which operand is a constant, the memo's storage.
    pub(crate) flag: WordFlag,
    /// A small operand: a field, a node column, a width.
    pub(crate) short: Short,
    /// A register, or what the flag says.
    pub(crate) a: WordSlot,
    /// A register, or what the flag says.
    pub(crate) b: WordSlot,
    /// A constant, a pool offset, an entry or a site.
    pub(crate) x: WordSlot,
}

/// The operation of a word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Op {
    /// Ends a program: the candidate is done. CodeEntry 0 holds one, so no entry is 0.
    #[default]
    End,
    /// `EachKind`.
    EachKind,
    /// `EachMatch` over the node table.
    EachMatchNode,
    /// `EachMatch` over a filled relation, one column.
    EachMatchFilled,
    /// `EachMatch` over a filled relation, several columns.
    EachMatchFilledMany,
    /// `EachMatch` over a made table.
    EachMatchMade,
    /// `EachRange`.
    EachRange,
    /// `EachRow` over the node table.
    EachRowNode,
    /// `EachRow` over a filled relation.
    EachRowFilled,
    /// `EachRow` over a made table.
    EachRowMade,
    /// `Children`.
    Children,
    /// `Children` of one field.
    ChildrenOfField,
    /// `Axis`.
    Axis,
    /// `Within`.
    Within,
    /// `Nearest` over the tree.
    Nearest,
    /// `Nearest` over derived edges.
    NearestDerived,
    /// `Closure`.
    Closure,
    /// `Read` of a node-table column.
    ReadNode,
    /// `Read` of a filled column.
    ReadFilled,
    /// `Read` of a made table's column.
    ReadMade,
    /// `Test`, both operands by the flag.
    Test,
    /// `Test` of a register for equality with a constant.
    TestEqualConstant,
    /// `Test` of a register for inequality with a constant.
    TestNotEqualConstant,
    /// `Test` of two registers for equality.
    TestEqualRegister,
    /// `Test` of two registers for inequality.
    TestNotEqualRegister,
    /// `Test` of a register against a constant set.
    TestInConstants,
    /// `Call`, the meaning in the short operand.
    Call,
    /// `Render`.
    Render,
    /// `Let`.
    Let,
    /// `Exists`.
    Exists,
    /// `Or`.
    Or,
    /// `Fork`.
    Fork,
    /// `Yield`.
    Yield,
    /// `Latest`.
    Latest,
    /// `Emit`.
    Emit,
    /// A pipeline's `Scan`.
    StageScan,
    /// A pipeline's `Probe`, or a `Use` of a pattern in table form.
    StageProbe,
    /// A pipeline's `Latest`.
    StageLatest,
    /// A pipeline's `Read`.
    StageRead,
    /// A pipeline's `Keep`.
    StageKeep,
    /// A pipeline's `Call`.
    StageCall,
    /// A pipeline's `Product`.
    StageProduct,
    /// A `Use` of a pattern in call form.
    StageUse,
    /// A pipeline's `Render`.
    StageRender,
    /// A pipeline's `Walk`.
    StageWalk,
    /// The end of a pipeline: its row written to the table.
    StageWrite,
}

/// The variant bits of a word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WordFlag(pub(crate) u8);
/// The small operand of a word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Short(pub(crate) u16);
/// A wide operand of a word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WordSlot(pub(crate) u32);

/// Where a program starts in the arena.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CodeEntry(pub(crate) u32);
/// A memo site, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MemoSite(pub(crate) u32);
/// A per-file index, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct IndexSite(pub(crate) u32);
/// An emptiness decision, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NeedSite(pub(crate) u32);
/// A place in the pool: where a list or a record starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PoolAt(pub(crate) u32);
/// A constant, by its number among the code's constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ConstantNumber(pub(crate) u32);
/// A relation, by its number among the relations the code reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RelationNumber(pub(crate) u32);

/// How many of each numbered thing a worker's state holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StateCounts {
    /// Registers.
    pub(crate) registers: WordSlot,
    /// Memo sites.
    pub(crate) memos: WordSlot,
    /// Per-file indexes.
    pub(crate) indexes: WordSlot,
    /// Made tables.
    pub(crate) tables: WordSlot,
    /// Emptiness decisions.
    pub(crate) needs: WordSlot,
    /// Relations read.
    pub(crate) relations: WordSlot,
    /// Plan points.
    pub(crate) points: WordSlot,
    /// Units.
    pub(crate) units: WordSlot,
}

/// One rule's entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleCode {
    /// The rule.
    pub(crate) rule: SelectedRuleOrdinal,
    /// Its branches: the emptiness decision each waits on and where it starts.
    pub(crate) branches: Box<[BranchCode]>,
    /// What each column of the rows it emits holds.
    pub(crate) columns: RuleColumns,
}

/// One branch of a rule: the emptiness decision it waits on and where it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BranchCode {
    /// The decision.
    pub(crate) need: NeedSite,
    /// The entry.
    pub(crate) entry: CodeEntry,
}

/// A made table as the executor fills it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TableCode {
    /// The fill, as its code.
    pub(crate) fill: Short,
    /// The fill's operands, a pool offset.
    pub(crate) operands: WordSlot,
    /// The emptiness decision it waits on.
    pub(crate) need: NeedSite,
}

/// The executable form of a rule set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProgramCode {
    /// Every program, one arena.
    pub(crate) words: Vec<CodeWord>,
    /// Operand lists, each once.
    pub(crate) pool: Vec<WordSlot>,
    /// Number constants and base-word codes.
    pub(crate) constants: Vec<CellWord>,
    /// Each emptiness decision as a formula over the pool.
    pub(crate) needs: Vec<WordSlot>,
    /// The made tables.
    pub(crate) tables: Vec<TableCode>,
    /// The rules.
    pub(crate) rules: Vec<RuleCode>,
    /// The relations read, by the number the words name.
    pub(crate) relations: Vec<Relation>,
    /// Each per-file index: what it is over.
    pub(crate) indexes: Vec<IndexCode>,
    /// The plan point each word is counted at, one per word; `WordSlot(u32::MAX)` for none.
    pub(crate) points: Vec<WordSlot>,
    /// The units the plan points belong to, each a run of points.
    pub(crate) units: Vec<UnitCode>,
    /// The buckets a measuring scratch sizes: every constant bucket the lowering costed.
    pub(crate) sizes: Vec<SizeSite>,
    /// What a worker's state must hold.
    pub(crate) counts: StateCounts,
}

/// A per-file index: the rows of a source by some of its columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexCode {
    /// The source: 0 the node table, 1 a filled relation, 2 a made table.
    pub(crate) source: Short,
    /// The relation's or the table's number.
    pub(crate) number: WordSlot,
    /// The key columns, a pool list of column codes.
    pub(crate) columns: WordSlot,
    /// The column the rows of a key are sorted by, as a column code; `WordSlot(u32::MAX)` for none.
    pub(crate) ordered: WordSlot,
}

/// A unit of the statistics: a run of plan points and the form it was lowered in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnitCode {
    /// What the unit is named by.
    pub(crate) key: UnitKey,
    /// The form it is in.
    pub(crate) form: MeasuredForm,
    /// Its first plan point.
    pub(crate) first: WordSlot,
    /// How many plan points it has.
    pub(crate) points: WordSlot,
}

/// A constant bucket whose rows a measuring scratch counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SizeSite {
    /// The relation's number.
    pub(crate) relation: WordSlot,
    /// The column, as a column code.
    pub(crate) column: Short,
    /// The value, as a constant's number.
    pub(crate) value: WordSlot,
}

/// What a unit is named by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum UnitKey {
    /// A definition's pattern: its node, its bound columns, its collected columns.
    Pattern(NodeDigest, ColumnMask, ColumnMask),
    /// A rule's own program: the node of the step it emits.
    Rule(NodeDigest),
    /// A made table's fill: its node and its fill's code.
    Table(NodeDigest, FillCode),
}

/// The code of a made table's fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FillCode(pub(crate) u16);

/// The form a pattern was measured in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MeasuredForm {
    /// Not a pattern, or a pattern in call form.
    #[default]
    Call,
    /// A pattern in table form.
    Table,
}
