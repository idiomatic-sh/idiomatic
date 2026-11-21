//! Test support of the `executing` concept: the fixture packs the bodies' cases run, each a rule over a made table written
//! as steps.

use crate::domain::code::types::ShapeChoice;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill,
    MadeTable, PackDefinition, PatternForm,
};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, PlacedStep, Polarity,
    ProgramStep, ReadColumn, Register, RegisterCount, RegisterList, StepBody, StepMemo,
    StepOperand, StepOperands, StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// The rule a case runs over the table of rows (1, 10), (2, 10), (3, 3): register 0 is the row, 1 its second cell.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Scene {
    /// The rows whose second cell is above 5, by an inline body, kept by that cell or not.
    Exists(Polarity, Kept),
    /// For each row its second cell, and 100: two inline bodies.
    OrInline,
    /// For each row what a pattern in call form yields for its second cell: the cell, when above 5.
    OrPattern,
    /// Each row's first cell emitted by one slot, and by a second slot when above 1.
    Fork,
    /// For each row, the second cells of the rows of table 1 whose first cell is the row's second cell: a pattern in table
    /// form, collected.
    OrTable,
    /// The rows whose second cell is a first cell of table 1: a pattern in table form, asked for a row.
    ExistsTable,
    /// The pattern in call form asked for the key 10 twice, by key, as a `Use` stage asks it back.
    AskedByKey,
    /// The rows for which an inline body over an `Or` has a row above 5: the body ends at its first row, with the `Or`
    /// still iterating.
    ExistsOverOr,
}

/// Whether a word keeps its body's answer by key.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Kept {
    /// Asked every time.
    No,
    /// Asked once per key.
    ByKey,
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

/// A register as an operand.
fn reg(number: u32) -> StepOperand {
    StepOperand::Register(Register(number))
}

/// A test that a register is above 5.
fn above_five(register: u32) -> ProgramStep {
    ProgramStep::Test {
        test: TestOperation::Greater,
        left: reg(register),
        right: StepOperand::Word(CellWord(5)),
    }
}

/// An emit of one register.
fn emit(register: u32) -> ProgramStep {
    ProgramStep::Emit {
        target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
        columns: StepOperands(Box::new([reg(register)])),
    }
}

/// A body that yields one operand as register 3.
fn yielding(from: StepOperand) -> StepBody {
    StepBody::Inline(program(vec![
        ProgramStep::Let {
            from,
            out: Register(3),
        },
        ProgramStep::Yield {
            row: RegisterList(Box::new([Register(3)])),
        },
    ]))
}

/// The steps of a scene's rule after its anchor and the read of the second cell into register 1.
fn tail(scene: Scene) -> Vec<ProgramStep> {
    let first_cell = ProgramStep::Read {
        source: StepRows::Made(MadeTableId(0)),
        column: ReadColumn::Made(MadeColumn(0)),
        row: Register(0),
        out: Register(2),
    };
    match scene {
        Scene::Exists(polarity, kept) => vec![
            ProgramStep::Exists {
                polarity,
                body: StepBody::Inline(program(vec![above_five(1)])),
                memo: match kept {
                    Kept::No => StepMemo::None,
                    Kept::ByKey => StepMemo::Keyed(RegisterList(Box::new([Register(1)]))),
                },
            },
            first_cell,
            emit(2),
        ],
        Scene::OrInline => vec![
            ProgramStep::Or {
                bodies: Box::new([yielding(reg(1)), yielding(StepOperand::Word(CellWord(100)))]),
                outputs: RegisterList(Box::new([Register(4)])),
                memo: StepMemo::None,
            },
            emit(4),
        ],
        Scene::OrPattern | Scene::AskedByKey => vec![
            ProgramStep::Or {
                bodies: Box::new([StepBody::Definition {
                    definition: DefinitionId(0),
                    bound: ColumnMask(Box::new([MadeColumn(0)])),
                    collected: ColumnMask(Box::new([MadeColumn(1)])),
                    values: StepOperands(Box::new([reg(1)])),
                }]),
                outputs: RegisterList(Box::new([Register(4)])),
                memo: StepMemo::None,
            },
            emit(4),
        ],
        Scene::ExistsOverOr => vec![
            ProgramStep::Exists {
                polarity: Polarity::Present,
                body: StepBody::Inline(program(vec![
                    ProgramStep::Or {
                        bodies: Box::new([
                            yielding(reg(1)),
                            yielding(StepOperand::Word(CellWord(100))),
                        ]),
                        outputs: RegisterList(Box::new([Register(4)])),
                        memo: StepMemo::None,
                    },
                    above_five(4),
                ])),
                memo: StepMemo::None,
            },
            first_cell,
            emit(2),
        ],
        Scene::OrTable => vec![
            ProgramStep::Or {
                bodies: Box::new([StepBody::Definition {
                    definition: DefinitionId(1),
                    bound: ColumnMask(Box::new([MadeColumn(0)])),
                    collected: ColumnMask(Box::new([MadeColumn(1)])),
                    values: StepOperands(Box::new([reg(1)])),
                }]),
                outputs: RegisterList(Box::new([Register(4)])),
                memo: StepMemo::None,
            },
            emit(4),
        ],
        Scene::ExistsTable => vec![
            ProgramStep::Exists {
                polarity: Polarity::Present,
                body: StepBody::Definition {
                    definition: DefinitionId(1),
                    bound: ColumnMask(Box::new([MadeColumn(0)])),
                    collected: ColumnMask(Box::new([])),
                    values: StepOperands(Box::new([reg(1)])),
                },
                memo: StepMemo::None,
            },
            first_cell,
            emit(2),
        ],
        Scene::Fork => vec![
            first_cell,
            ProgramStep::Fork {
                slots: Box::new([
                    program(vec![emit(2)]),
                    program(vec![
                        ProgramStep::Test {
                            test: TestOperation::Greater,
                            left: reg(2),
                            right: StepOperand::Word(CellWord(1)),
                        },
                        emit(2),
                    ]),
                ]),
            },
        ],
    }
}

/// The scene's rule as a pack: one rule, one table, and one definition whose pattern in call form yields its bound cell
/// when it is above 5 (formals: registers 5 and 6).
pub(crate) fn pack(scene: Scene) -> LoweredPack {
    let made = StepRows::Made(MadeTableId(0));
    let mut steps = vec![
        ProgramStep::EachRow {
            source: made,
            out: Register(0),
        },
        ProgramStep::Read {
            source: made,
            column: ReadColumn::Made(MadeColumn(1)),
            row: Register(0),
            out: Register(1),
        },
    ];
    steps.extend(tail(scene));
    let made_table = |node: u8| MadeTable {
        node: NodeDigest([node; 16]),
        fill: MadeFill::Constant(ConstantRows(Vec::new())),
        width: ColumnCount(2),
        indexes: Vec::new(),
        emptiness: Emptiness::Unknown,
    };
    let in_table = |collected: &[u16]| DefinitionPattern {
        bound: ColumnMask(Box::new([MadeColumn(0)])),
        collected: ColumnMask(collected.iter().map(|column| MadeColumn(*column)).collect()),
        shape: ShapeChoice::Decided,
        form: PatternForm::Table(MadeTableId(1)),
    };
    let tabled = PackDefinition {
        node: NodeDigest([5; 16]),
        formals: RegisterList(Box::new([Register(7), Register(8)])),
        emptiness: Emptiness::Unknown,
        patterns: vec![in_table(&[1]), in_table(&[])],
    };
    LoweredPack {
        definitions: vec![
            PackDefinition {
                node: NodeDigest([3; 16]),
                formals: RegisterList(Box::new([Register(5), Register(6)])),
                emptiness: Emptiness::Unknown,
                patterns: vec![DefinitionPattern {
                    bound: ColumnMask(Box::new([MadeColumn(0)])),
                    collected: ColumnMask(Box::new([MadeColumn(1)])),
                    shape: ShapeChoice::Decided,
                    form: PatternForm::Call(program(vec![
                        above_five(5),
                        ProgramStep::Let {
                            from: reg(5),
                            out: Register(6),
                        },
                        ProgramStep::Yield {
                            row: RegisterList(Box::new([Register(6)])),
                        },
                    ])),
                }],
            },
            tabled,
        ],
        tables: vec![made_table(2), made_table(4)],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: program(steps),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(9),
        regexes: Vec::new(),
        texts: Vec::new(),
        reads: Vec::new(),
    }
}
