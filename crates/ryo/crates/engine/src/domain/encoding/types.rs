//! The types of the `encoding` concept: what an encoding has written so far and what it has numbered. Crate-private; dropped
//! when the code is answered.

use crate::domain::code::records::types::{BoundFormal, CollectedRead, PooledOperand};
use crate::domain::code::types::{
    CodeEntry, CodeWord, ConstantNumber, IndexSite, MemoSite, NeedSite, PoolAt, ProgramCode,
    SizeSite, WordFlag, WordSlot,
};
use crate::domain::lowered::types::{DefinitionPattern, LoweredPack, PackDefinition};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, MadeTableId, RegisterCount, StepOperands,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::collections::{HashMap, HashSet};

/// An encoding in progress.
#[derive(Debug)]
pub(crate) struct Encoding<'p> {
    /// What is encoded.
    pub(crate) pack: &'p LoweredPack,
    /// What is written so far.
    pub(crate) code: ProgramCode,
    /// The rule being written: whose refusal a count past its width, or a fault of the encoding, is.
    pub(crate) rule: SelectedRuleOrdinal,
    /// Each constant's number.
    pub(crate) constants: HashMap<CellWord, ConstantNumber>,
    /// Each index's site, by what the index is over: an index is one site whoever asks for it.
    pub(crate) indexes: HashMap<IndexedBy, IndexSite>,
    /// Each pattern reached, by its definition and its place among the definition's patterns.
    pub(crate) patterns: HashMap<PatternKey, PatternCode>,
    /// The units reached and not yet written, in the order they were reached.
    pub(crate) reached: Vec<ReachedUnit>,
    /// How many of the reached units are written.
    pub(crate) written: Count,
    /// The places of the pool that wait for a pattern's entry.
    pub(crate) entries: Vec<EntryPlace>,
    /// What is known of each made table, by number.
    pub(crate) tables: Vec<TableProgress>,
    /// The place of each need site's record among the records.
    pub(crate) need_places: Vec<WordSlot>,
    /// The need records.
    pub(crate) need_records: Vec<WordSlot>,
    /// The places, among the records, of the slots that hold a record's place: moved by the number of sites at the end.
    pub(crate) need_offsets: Vec<WordSlot>,
    /// The need site of each definition reached by a formula.
    pub(crate) definition_needs: HashMap<DefinitionId, NeedSite>,
    /// The constant buckets noted so far: each is one size site.
    pub(crate) sizes: HashSet<SizeSite>,
    /// How many memo sites are numbered.
    pub(crate) memos: WordSlot,
    /// The plan points of the unit being written.
    pub(crate) unit: UnitPoints,
    /// How many registers the code names: the pack's, and the ones the encoding adds for a row handle no stage names.
    pub(crate) registers: RegisterCount,
}

/// Where rows are read from, as the code numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NumberedSource {
    /// The source: 0 the node table, 1 a filled relation, 2 a made table.
    pub(crate) source: WordSlot,
    /// The relation's or the table's number.
    pub(crate) number: WordSlot,
}

/// What an index is over: an index is one site for one of these, whoever asks for it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexedBy {
    /// The rows indexed.
    pub(crate) rows: NumberedSource,
    /// The key columns, each as its source's column code.
    pub(crate) key: Vec<WordSlot>,
    /// The column the rows of a key are sorted by, or `NO_SLOT`.
    pub(crate) sorted: WordSlot,
}

/// A pattern, by its definition and its place among the definition's patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PatternKey {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// The pattern's place.
    pub(crate) pattern: WordSlot,
}

/// What is numbered of a pattern in call form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatternCode {
    /// Where its answers are kept.
    pub(crate) memo: MemoSite,
    /// Where its program starts, once written.
    pub(crate) entry: Option<CodeEntry>,
    /// Its unit, once written.
    pub(crate) unit: Option<WordSlot>,
}

/// A unit reached from the rule being written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReachedUnit {
    /// A pattern in call form.
    Pattern(PatternKey),
    /// A made table.
    Table(MadeTableId),
    /// A definition's own emptiness formula, for the need site it was given.
    Need(DefinitionId, NeedSite),
}

/// A place of the pool that waits for a pattern's entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EntryPlace {
    /// Where the record of the use starts: its entry and its unit are written there.
    pub(crate) record: PoolAt,
    /// The pattern whose entry goes there.
    pub(crate) pattern: PatternKey,
}

/// What is known of a made table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TableProgress {
    /// No unit written so far reads it.
    #[default]
    Unreached,
    /// A unit reads it; its fill is written when the rule that reached it is.
    Reached,
}

/// The plan points of the unit being written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct UnitPoints {
    /// The unit's place among the code's units.
    pub(crate) unit: WordSlot,
    /// Its first plan point among the code's.
    pub(crate) first: WordSlot,
    /// How many plan points its words have named so far.
    pub(crate) reach: WordSlot,
}

/// A word and the plan point it is counted at, before it has its place in the arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlacedWord {
    /// The word.
    pub(crate) word: CodeWord,
    /// Its plan point among the code's, or `NO_SLOT`.
    pub(crate) point: WordSlot,
}

/// Which entry a step program is: what decides where a file-sized word may stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProgramRole {
    /// A rule's branch: its first step is its anchor, once per file.
    Branch,
    /// An inline body, a pattern in call form or a slot of a `Fork`: run per candidate or per key.
    Keyed,
}

/// A body with everything it reaches written, ready to be written as one record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReadyBody {
    /// Steps written in place.
    Inline(CodeEntry),
    /// A use of a pattern in call form.
    Call {
        /// The pattern.
        pattern: PatternKey,
        /// What is numbered of it.
        code: PatternCode,
        /// The need site of the pattern's definition.
        need: NeedSite,
        /// The caller's cells for its bound columns.
        formals: Vec<BoundFormal>,
    },
    /// A use of a pattern in table form.
    Table {
        /// The index of the definition's table by the bound columns.
        index: IndexSite,
        /// The caller's cells for its bound columns.
        values: Vec<PooledOperand>,
        /// The collected columns and where each goes.
        reads: Vec<CollectedRead>,
    },
}

/// A value a word holds in one slot: a register, or a constant's number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotValue {
    /// A register.
    Register(WordSlot),
    /// A constant, by number.
    Constant(WordSlot),
}

/// A use of a definition: the definition, the columns it binds and collects, and the caller's cells for the bound ones.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DefinitionUse<'u> {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// The columns that arrive bound.
    pub(crate) bound: &'u ColumnMask,
    /// The columns a row gives back.
    pub(crate) collected: &'u ColumnMask,
    /// The caller's cells for the bound columns, ascending.
    pub(crate) values: &'u StepOperands,
}

/// A slot's value and the flag bits that say what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlaggedSlot {
    /// The bits.
    pub(crate) flag: WordFlag,
    /// The value.
    pub(crate) slot: WordSlot,
}

/// A pool operand and the flag bits that say what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlaggedOperand {
    /// The bits.
    pub(crate) flag: WordFlag,
    /// The operand.
    pub(crate) operand: PooledOperand,
}

/// The definition a use names, its pattern of the use's columns, and the pattern's place.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UsedPattern<'p> {
    /// The definition.
    pub(crate) definition: &'p PackDefinition,
    /// Its pattern of the use's bound and collected columns.
    pub(crate) pattern: &'p DefinitionPattern,
    /// The pattern, by its definition and its place.
    pub(crate) key: PatternKey,
}
