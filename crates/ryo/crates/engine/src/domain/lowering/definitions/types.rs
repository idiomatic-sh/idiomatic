//! The types of the `definitions` part: a definition before it is placed, the patterns asked of it, and the pack's drafts.

use crate::domain::code::types::ShapeChoice;
use crate::domain::lowered::types::{Emptiness, MadeTable, PatternForm};
use crate::domain::lowering::costing::types::{SampleSizes, UseCosts};
use crate::domain::lowering::flattening::types::{FlatBranch, FlatVariable, RowColumn, TextNumber};
use crate::domain::lowering::nodes::types::{NodeHome, StepShape};
use crate::domain::steps::types::{
    ColumnMask, ConstantText, DefinitionId, MadeTableId, RegexId, Register, RegisterCount,
    StepRegex,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{LiteralText, StepId};
use ryo_values::domain::query_plans::types::NodeDigest;
use std::collections::{BTreeSet, HashMap};

/// How many rows a source is taken to hold in a file, by the fixed priors: an estimate that orders accesses, never a bound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RowEstimate(pub(crate) u64);

/// The columns a use asks a pattern by: the ones it binds, and the ones it collects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AskedColumns {
    /// The columns that arrive bound.
    pub(crate) bound: ColumnMask,
    /// The columns a row gives back.
    pub(crate) collected: ColumnMask,
}

/// One pattern asked of a definition, and how it is placed once it is.
#[derive(Debug)]
pub(crate) struct PatternDraft {
    /// The columns it is asked by.
    pub(crate) asked: AskedColumns,
    /// How it is evaluated; none until it is placed.
    pub(crate) form: Option<PatternForm>,
    /// Whether the shape of its plans left its form open: a flip candidate.
    pub(crate) shape: ShapeChoice,
}

/// A definition as flattened: its branches are written once and never changed after.
#[derive(Debug)]
pub(crate) struct DefinitionDraft {
    /// The merged plan node it is the rows of.
    pub(crate) node: NodeDigest,
    /// Where it was lowered from.
    pub(crate) home: NodeHome,
    /// What its columns hold.
    pub(crate) shape: StepShape,
    /// Its branches.
    pub(crate) branches: Vec<FlatBranch>,
    /// What a file must hold for it to have a row.
    pub(crate) emptiness: Emptiness,
    /// The joins of its own plan that state no key within the file: the steps a product stage of its table stands for.
    pub(crate) keyless: Vec<StepId>,
    /// One register per column.
    pub(crate) formals: Vec<Register>,
    /// Its rows in a file, by the fixed priors.
    pub(crate) rows: RowEstimate,
    /// The patterns asked of it, in the order they were first asked.
    pub(crate) patterns: Vec<PatternDraft>,
    /// Its table, once some pattern is in table form.
    pub(crate) table: Option<MadeTableId>,
}

/// A pattern of a definition, by its place among the definition's patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatternPlace(pub(crate) usize);

/// A pattern that waits to be placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaitingPattern {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// The pattern's place among the definition's.
    pub(crate) pattern: PatternPlace,
}

/// The pack as it is being lowered.
#[derive(Debug, Default)]
pub(crate) struct Drafts {
    /// The definitions, by number: a definition reads only definitions before it.
    pub(crate) definitions: Vec<DefinitionDraft>,
    /// The definition of each merged plan node that has one.
    pub(crate) defined: HashMap<NodeDigest, DefinitionId>,
    /// The made tables, by number.
    pub(crate) tables: Vec<MadeTable>,
    /// The made table of each merged plan node that makes one: one table a node, whoever reads it.
    pub(crate) made: HashMap<NodeDigest, MadeTableId>,
    /// Where each made table was lowered from, by table.
    pub(crate) table_homes: Vec<NodeHome>,
    /// The columns of its step's row a definition holds, in order, for each definition that holds fewer than the step has:
    /// one whose every reader across the pack is a projection, and no reader asks the others.
    pub(crate) held: HashMap<DefinitionId, Vec<RowColumn>>,
    /// The read of each merged plan node that is one read of a relation with its pins: what each reader that takes the
    /// node's branches reads in place (`tables::views`).
    pub(crate) views: HashMap<NodeDigest, Flattened>,
    /// The patterns asked and not yet placed.
    pub(crate) waiting: Vec<WaitingPattern>,
    /// The next variable no branch names.
    pub(crate) variables: FlatVariable,
    /// How many registers the programs name.
    pub(crate) registers: RegisterCount,
    /// The constant texts, by number.
    pub(crate) texts: Vec<ConstantText>,
    /// What each numbered text is, as written: a kind is found by its name.
    pub(crate) written: HashMap<TextNumber, ConstantText>,
    /// The number of each constant text.
    pub(crate) text_numbers: HashMap<LiteralText, TextNumber>,
    /// The regular expressions, by number.
    pub(crate) regexes: Vec<StepRegex>,
    /// The number of each regular expression.
    pub(crate) regex_numbers: HashMap<LiteralText, RegexId>,
    /// Every relation some branch reads.
    pub(crate) reads: BTreeSet<Relation>,
    /// What each use of a definition asked so far costs, by the definition's own plan for the columns bound: a
    /// definition's branches never change once it is made, so a use is costed once for the pack.
    pub(crate) costs: UseCosts,
    /// What a sample measured the buckets of a file at; none for a load given no statistics.
    pub(crate) sized: Option<SampleSizes>,
}

/// What a step's flattening gives a definition: its branches, its emptiness and the keyless joins inlined in it.
#[derive(Debug, Default)]
pub(crate) struct Flattened {
    /// The branches.
    pub(crate) branches: Vec<FlatBranch>,
    /// What a file must hold for a row.
    pub(crate) emptiness: Emptiness,
    /// The joins flattened into the branches that state no key within the file.
    pub(crate) keyless: Vec<StepId>,
}
