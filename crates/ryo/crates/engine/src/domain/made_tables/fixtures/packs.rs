//! The packs of the fill side's cases: the smallest parts of a lowered pack, the stages the cases compose, and the code the
//! contract's encoding writes for a pack of tables.

use crate::domain::code::types::ProgramCode;
use crate::domain::encoding::service::encoded;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::MissingCode;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable, PackDefinition,
};
use crate::domain::steps::stages::types::{
    FillStage, Pipeline, PlacedStage, ProbeRange, TreeWalk, UseMode,
};
use crate::domain::steps::types::{
    ColumnList, ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, MatchColumns,
    PlacedStep, ProgramStep, ReadColumn, Register, RegisterCount, RegisterList, StepOperand,
    StepOperands, StepProgram, StepRegex, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
use ryo_relations::domain::relations::types::{Column, FileId, NodeColumn, Relation};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// The relation the world's language fills: five columns, of which the cases read the first two.
pub(crate) const TOLD: Relation = Relation::TokenLink;

/// The first column of the relation.
pub(crate) const FIRST: Column = Column::TokenLink(TokenLinkColumn::Source);

/// The second column of the relation.
pub(crate) const SECOND: Column = Column::TokenLink(TokenLinkColumn::Target);

/// The file the world scans.
pub(crate) const FILE: FileId = FileId(7);

/// The file's bytes.
pub(crate) const TEXT: &[u8] = b"ab cd ef";

/// The regular expressions of the cases' rule set, by number: one a `Keep` matches with, one a `Replace` replaces, one a
/// source search finds words with, and one whose named group it captures.
pub(crate) const REGEXES: [&str; 4] = ["^[ac]", "[bd]", "[a-z]+", "(?P<first>[a-z])[a-z]"];

/// The texts the cases' rule set knows as constants, in the order of their words from word 1: two globs, a token's own
/// text, a replacement, two slot patterns and a group's name.
pub(crate) const TEXTS: [&str; 9] = ["a*", "?d", "ef", "_", "{a}b", "{x}{y}", "first", "a", "f"];

/// A register as an operand.
pub(crate) const fn of(number: u32) -> StepOperand {
    StepOperand::Register(Register(number))
}

/// A cell as an operand.
pub(crate) const fn word(bits: u64) -> StepOperand {
    StepOperand::Word(CellWord(bits))
}

/// Operands, in order.
pub(crate) fn operands<const N: usize>(each: [StepOperand; N]) -> StepOperands {
    StepOperands(Box::new(each))
}

/// A pipeline of the stages, writing `row`.
pub(crate) fn pipeline<const N: usize>(each: Vec<FillStage>, row: [StepOperand; N]) -> Pipeline {
    Pipeline {
        stages: each
            .into_iter()
            .map(|stage| PlacedStage { point: None, stage })
            .collect(),
        row: operands(row),
    }
}


/// A made table of `width` columns with the fill.
pub(crate) fn table(node: u8, width: usize, fill: MadeFill) -> MadeTable {
    MadeTable {
        node: NodeDigest([node; 16]),
        fill,
        width: ColumnCount(width),
        indexes: vec![ColumnList(Box::new([MadeColumn(0)]))],
        emptiness: Emptiness::Unknown,
    }
}

/// The code of a pack of the tables: a rule for each table reads it, so the encoding writes each.
///
/// # Errors
///
/// The encoding refuses the pack.
pub(crate) fn coded(
    tables: Vec<MadeTable>,
    definitions: Vec<PackDefinition>,
) -> Result<ProgramCode, FilledError> {
    let rules = (0..tables.len())
        .map(|at| {
            let number = u32::try_from(at).unwrap_or(u32::MAX);
            let steps = [
                ProgramStep::EachRow {
                    source: StepRows::Made(MadeTableId(number)),
                    out: Register(900_u32.saturating_add(number)),
                },
                ProgramStep::Emit {
                    target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
                    columns: operands([]),
                },
            ];
            LoweredRule {
                rule: SelectedRuleOrdinal(number),
                node: NodeDigest([u8::try_from(at).unwrap_or(u8::MAX); 16]),
                branches: vec![Branch {
                    emptiness: Emptiness::Unknown,
                    program: StepProgram(
                        steps
                            .into_iter()
                            .map(|step| PlacedStep { point: None, step })
                            .collect(),
                    ),
                }],
                columns: RuleColumns::default(),
            }
        })
        .collect();
    let pack = LoweredPack {
        definitions,
        tables,
        rules,
        regexes: REGEXES
            .iter()
            .map(|source| StepRegex((*source).into()))
            .collect(),
        registers: RegisterCount(1000),
        ..LoweredPack::default()
    };
    encoded(&pack).map_err(|_| FilledError::Code {
        named: MissingCode::Word,
    })
}


/// A scan of the relation into a register.
pub(crate) fn scan(out: u32) -> FillStage {
    FillStage::Scan {
        source: StepRows::Filled(TOLD),
        columns: MatchColumns::default(),
        values: operands([]),
        out: Register(out),
    }
}

/// A read of one of the relation's columns.
pub(crate) fn read(column: Column, row: u32, out: u32) -> FillStage {
    FillStage::Read {
        source: StepRows::Filled(TOLD),
        column: ReadColumn::Filled(column),
        row: Register(row),
        out: Register(out),
    }
}



/// A table of the rows one pipeline writes, repeats kept.
pub(crate) fn rows(width: usize, piped: Pipeline) -> MadeTable {
    table(
        91,
        width,
        MadeFill::Rows(Box::new([piped]), RepeatedRows::Kept),
    )
}

/// A probe of the relation by its first column with the cell of a register.
pub(crate) fn probe(value: u32, range: Option<ProbeRange>, mode: UseMode, out: u32) -> FillStage {
    FillStage::Probe {
        source: StepRows::Filled(TOLD),
        columns: MatchColumns(Box::new([ReadColumn::Filled(FIRST)])),
        values: operands([of(value)]),
        range,
        mode,
        out: Register(out),
    }
}

/// A scan of the nodes of `relation`, with each node's kind read.
pub(crate) fn scan_nodes(relation: Relation) -> [FillStage; 2] {
    [
        FillStage::Scan {
            source: StepRows::Node(relation),
            columns: MatchColumns::default(),
            values: operands([]),
            out: Register(0),
        },
        node_read(relation, NodeColumn::Kind, 0, 1),
    ]
}

/// A read of a node row's column.
pub(crate) fn node_read(relation: Relation, column: NodeColumn, row: u32, out: u32) -> FillStage {
    FillStage::Read {
        source: StepRows::Node(relation),
        column: ReadColumn::Node(column),
        row: Register(row),
        out: Register(out),
    }
}

/// A keep of the rows whose register holds the cell.
pub(crate) fn keep_equal(register: u32, cell: u64) -> FillStage {
    FillStage::Keep {
        test: TestOperation::Equal,
        left: of(register),
        right: word(cell),
    }
}

/// A walk from a register.
pub(crate) fn walk(relation: Relation, walked: TreeWalk, start: u32, out: u32) -> FillStage {
    FillStage::Walk {
        relation,
        walk: walked,
        start: Register(start),
        out: Register(out),
    }
}

/// A use of the one definition's pattern in call form, bound by a register.
pub(crate) fn used(value: u32, mode: UseMode, outputs: &[u32]) -> FillStage {
    FillStage::Use {
        definition: DefinitionId(0),
        bound: ColumnMask(Box::new([MadeColumn(0)])),
        collected: ColumnMask(match mode {
            UseMode::Rows => Box::new([MadeColumn(1)]),
            UseMode::Present | UseMode::Absent => Box::new([]),
        }),
        values: operands([of(value)]),
        mode,
        outputs: RegisterList(outputs.iter().map(|each| Register(*each)).collect()),
    }
}

