//! The loop's bench workloads: rule sets of one rule each, written as steps and encoded by the contract's encoding, whose
//! rule reads every row of a table of constants and runs one kind of word for each row, several times over. The bench
//! scans a file with each through the scan use case and divides the time by the steps the scan counted; a word's own
//! cost is what its rule set costs over the rule set that only reads the rows.

use crate::domain::code::types::ShapeChoice;
use crate::domain::encoding::service::encoded;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill,
    MadeTable, PackDefinition, PatternForm,
};
use crate::domain::made_tables::fixtures::packs::{of, operands, word};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    ColumnList, ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, MatchColumns,
    PlacedStep, Polarity, ProgramStep, ReadColumn, Register, RegisterCount, RegisterList, StepBody,
    StepMemo, StepOperand, StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// What a workload's rule does for each row of the table.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::dispatch::fixtures::workloads::{LOOP_BENCHES, LoopBench};
///
/// assert!(LOOP_BENCHES.contains(&LoopBench::Rows));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopBench {
    /// Nothing: the rows are read and the candidate ends. What every other workload costs over this is its words'.
    Rows,
    /// Reads the row's cell, `COPIES` times.
    Read,
    /// Tests a register against itself, `COPIES` times: the test holds.
    Test,
    /// Copies a register, `COPIES` times.
    Let,
    /// Asks an inline body of one `Yield` whether it has a row, `COPIES` times.
    ExistsInline,
    /// Asks a pattern in call form for one constant key, `COPIES` times: the first ask of the file evaluates it and
    /// every other is the memo's answer.
    CallHit,
    /// Asks a pattern in call form for the row's own key, once: every ask evaluates the pattern, whose program is one
    /// `Yield`, and keeps its answer: the cost of an evaluation, its `Yield` in it.
    CallMiss,
    /// Emits the row, once.
    Emit,
    /// Reads the table by key, once: the rows whose cell is the row's own, which is one row. A read by key through the
    /// seam, its cursor, and the candidate's end.
    Match,
}

/// How many times a workload's rule runs its word for a row.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::dispatch::fixtures::workloads::COPIES;
///
/// assert_eq!(COPIES, 8);
/// ```
pub const COPIES: usize = 8;

/// Every workload the bench times.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::dispatch::fixtures::workloads::LOOP_BENCHES;
///
/// assert_eq!(LOOP_BENCHES.len(), 9);
/// ```
pub const LOOP_BENCHES: &[LoopBench] = &[
    LoopBench::Rows,
    LoopBench::Read,
    LoopBench::Test,
    LoopBench::Let,
    LoopBench::ExistsInline,
    LoopBench::CallHit,
    LoopBench::CallMiss,
    LoopBench::Emit,
    LoopBench::Match,
];

/// A workload's name in the bench's measures.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::dispatch::fixtures::workloads::{LoopBench, loop_name};
///
/// assert_eq!(loop_name(LoopBench::CallHit), "call_hit");
/// ```
#[must_use]
pub const fn loop_name(bench: LoopBench) -> &'static str {
    match bench {
        LoopBench::Rows => "rows",
        LoopBench::Read => "read",
        LoopBench::Test => "test",
        LoopBench::Let => "let",
        LoopBench::ExistsInline => "exists_inline",
        LoopBench::CallHit => "call_hit",
        LoopBench::CallMiss => "call_miss",
        LoopBench::Emit => "emit",
        LoopBench::Match => "match",
    }
}

/// A program of steps with no plan point.
fn program(steps: Vec<ProgramStep>) -> StepProgram {
    StepProgram(
        steps
            .into_iter()
            .map(|step| PlacedStep { point: None, step })
            .collect(),
    )
}

/// An `Exists` over the workloads' one pattern in call form, asked for `value`.
fn asked(value: StepOperand) -> ProgramStep {
    ProgramStep::Exists {
        polarity: Polarity::Present,
        body: StepBody::Definition {
            definition: DefinitionId(0),
            bound: ColumnMask(Box::new([MadeColumn(0)])),
            collected: ColumnMask(Box::new([])),
            values: operands([value]),
        },
        memo: StepMemo::None,
    }
}

/// The rule set of a workload over a table of `rows` constants: one rule, and for the call workloads one definition
/// with one pattern in call form bound by its one column.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::domain::dispatch::fixtures::workloads::{LoopBench, loop_programs};
///
/// let programs = loop_programs(LoopBench::Read, 100).expect("the pack encodes");
/// assert_eq!(accepted(&programs).len(), 1);
/// ```
#[must_use]
pub fn loop_programs(bench: LoopBench, rows: usize) -> Option<Programs> {
    let table = StepRows::Made(MadeTableId(0));
    let cells = (0..u64::try_from(rows).ok()?)
        .map(|cell| -> Box<[CellWord]> { Box::new([CellWord(cell)]) });
    let constants = MadeTable {
        node: NodeDigest([90; 16]),
        fill: MadeFill::Constant(ConstantRows(cells.collect())),
        width: ColumnCount(1),
        indexes: vec![ColumnList(Box::new([MadeColumn(0)]))],
        emptiness: Emptiness::Unknown,
    };
    let read = || ProgramStep::Read {
        source: table,
        column: ReadColumn::Made(MadeColumn(0)),
        row: Register(0),
        out: Register(1),
    };
    let several = |step: &dyn Fn() -> ProgramStep| -> Vec<ProgramStep> {
        (0..COPIES).map(|_| step()).collect()
    };
    let inline = || ProgramStep::Exists {
        polarity: Polarity::Present,
        body: StepBody::Inline(program(vec![ProgramStep::Yield {
            row: RegisterList(Box::new([])),
        }])),
        memo: StepMemo::None,
    };
    let (words, emitted): (Vec<ProgramStep>, usize) = match bench {
        LoopBench::Rows => (Vec::new(), 0),
        LoopBench::Read => (several(&read), 0),
        LoopBench::Test => {
            let mut all = vec![read()];
            all.extend(several(&|| ProgramStep::Test {
                test: TestOperation::Equal,
                left: of(1),
                right: of(1),
            }));
            (all, 0)
        }
        LoopBench::Let => (
            several(&|| ProgramStep::Let {
                from: of(0),
                out: Register(2),
            }),
            0,
        ),
        LoopBench::ExistsInline => (several(&inline), 0),
        LoopBench::CallHit => (several(&|| asked(word(7))), 0),
        LoopBench::CallMiss => (vec![asked(of(0))], 0),
        LoopBench::Match => (
            vec![
                read(),
                ProgramStep::EachMatch {
                    source: table,
                    columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))])),
                    values: operands([of(1)]),
                    out: Register(3),
                },
            ],
            0,
        ),
        LoopBench::Emit => (
            vec![ProgramStep::Emit {
                target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
                columns: operands([of(0)]),
            }],
            1,
        ),
    };
    let mut steps = vec![ProgramStep::EachRow {
        source: table,
        out: Register(0),
    }];
    steps.extend(words);
    let pattern = DefinitionPattern {
        bound: ColumnMask(Box::new([MadeColumn(0)])),
        collected: ColumnMask(Box::new([])),
        form: PatternForm::Call(program(vec![ProgramStep::Yield {
            row: RegisterList(Box::new([])),
        }])),
        shape: ShapeChoice::Decided,
    };
    let pack = LoweredPack {
        definitions: vec![PackDefinition {
            node: NodeDigest([80; 16]),
            formals: RegisterList(Box::new([Register(5)])),
            emptiness: Emptiness::Unknown,
            patterns: vec![pattern],
        }],
        tables: vec![constants],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: program(steps),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number; emitted]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(8),
        ..LoweredPack::default()
    };
    Some(Programs(ProgramsEngine {
        code: encoded(&pack).ok()?,
        ..ProgramsEngine::default()
    }))
}
