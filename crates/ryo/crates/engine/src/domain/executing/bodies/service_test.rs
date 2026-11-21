//! The bodies' cases, each a rule over a made table written as steps and encoded by the straight encoding: an inline body
//! asked for a row, present and absent; an answer kept by key; an `Or` over inline bodies; a pattern in call form, asked
//! through the memo and run once per key; a `Fork`.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::ShapeChoice;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::service::run as run_program;
use crate::domain::executing::types::RunMode;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill,
    MadeTable, PackDefinition, PatternForm,
};
use crate::domain::programs::fixtures::{framed, state, table};
use crate::domain::programs::types::ProgramsEngine;
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
use std::borrow::Cow;

/// The rule a case runs over the table of rows (1, 10), (2, 10), (3, 3): register 0 is the row, 1 its second cell.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// The rows whose second cell is above 5, by an inline body, kept by that cell or not.
    Exists(Polarity, Kept),
    /// For each row its second cell, and 100: two inline bodies.
    OrInline,
    /// For each row what a pattern in call form yields for its second cell: the cell, when above 5.
    OrPattern,
    /// Each row's first cell emitted by one slot, and by a second slot when above 1.
    Fork,
    /// The rows for which an inline body over an `Or` has a row above 5: the body ends at its first row, with the `Or`
    /// still iterating.
    ExistsOverOr,
}

/// Whether a word keeps its body's answer by key.
#[derive(Clone, Copy, Debug)]
enum Kept {
    /// Asked every time.
    No,
    /// Asked once per key.
    ByKey,
}

type Input = Scene;
/// The cells found; then 99, the steps, the answers kept, and the staged cells, yielded cells and cursors left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_inline_body_with_a_row_lets_the_candidate_go_on"),
        word: Word::Succeeds,
        input: || Scene::Exists(Polarity::Present, Kept::No),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 99, 18, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("absent_keeps_the_candidates_whose_body_has_no_row"),
        word: Word::Succeeds,
        input: || Scene::Exists(Polarity::Absent, Kept::No),
        expect: Expect::Value(Cow::Borrowed(&[3, 99, 15, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("an_answer_kept_by_key_is_asked_once_per_key"),
        word: Word::Succeeds,
        input: || Scene::Exists(Polarity::Present, Kept::ByKey),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 99, 16, 2, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("an_or_gives_every_row_of_every_body"),
        word: Word::Succeeds,
        input: || Scene::OrInline,
        expect: Expect::Value(Cow::Borrowed(&[
            10, 100, 10, 100, 3, 100, 99, 31, 0, 0, 0, 0, 0,
        ])),
    },
    Case {
        name: CaseName("a_pattern_in_call_form_runs_once_per_key"),
        word: Word::Succeeds,
        input: || Scene::OrPattern,
        expect: Expect::Value(Cow::Borrowed(&[10, 10, 99, 15, 2, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_fork_runs_each_slot_from_the_same_registers"),
        word: Word::Succeeds,
        input: || Scene::Fork,
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 2, 3, 3, 99, 23, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_body_that_ends_at_its_first_row_leaves_no_staged_row"),
        word: Word::Succeeds,
        input: || Scene::ExistsOverOr,
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 3, 99, 38, 0, 0, 0, 0, 0])),
    },
];

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
        Scene::OrPattern => vec![
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
fn pack(scene: Scene) -> LoweredPack {
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
    LoweredPack {
        definitions: vec![PackDefinition {
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
        }],
        tables: vec![MadeTable {
            node: NodeDigest([2; 16]),
            fill: MadeFill::Constant(ConstantRows(Vec::new())),
            width: ColumnCount(2),
            indexes: Vec::new(),
            emptiness: Emptiness::Unknown,
        }],
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
        registers: RegisterCount(7),
        regexes: Vec::new(),
        texts: Vec::new(),
        reads: Vec::new(),
    }
}

fn subject(scene: Input) -> Result<Value, Failure> {
    let code = encoded(&pack(scene)).map_err(|_| ())?;
    let entry = code
        .rules
        .first()
        .and_then(|rule| rule.branches.first())
        .map(|branch| branch.entry)
        .ok_or(())?;
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = state(7);
    table(&mut held, 0, 2, &[&[1, 10], &[2, 10], &[3, 3]]);
    let (_rows, found) = framed(&programs, &mut held, |frame| {
        run_program(frame, entry, RunMode::Rows)
    });
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.extend([
        99,
        held.work.steps.0,
        u64::try_from(held.memos.entries.len()).unwrap_or(u64::MAX),
        u64::try_from(held.staged.len()).unwrap_or(u64::MAX),
        u64::try_from(held.yielded.len()).unwrap_or(u64::MAX),
        u64::try_from(held.frames.len()).unwrap_or(u64::MAX),
        held.work.unbuilt.0,
    ]);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
