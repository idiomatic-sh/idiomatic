//! The loop's cases, over fixture packs written as steps and encoded by the straight encoding: a rule's branch walks a
//! made table, reads, tests and emits; what the loop counts; a word it does not run ends each candidate and is counted.

use super::service::run as run_branch;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::encoding::service::encoded;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
};
use crate::domain::programs::fixtures::{framed, state, table};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{
    CallMeaning, EmitTarget, MadeColumn, MadeTableId, PlacedStep, ProgramStep, ReadColumn,
    Register, RegisterCount, RegisterList, StepOperand, StepOperands, StepProgram, StepRows,
    TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The program a case runs over the table of rows (1, 10), (2, 3), (3, 7).
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// Each row whose second cell is above 5 is emitted as its first cell and the file.
    ReadTestEmit,
    /// The same with a word the loop does not run before the emit.
    WithAnUnbuiltWord,
    /// The same with the row emitted twice: the rows a run answers are its emits, not its ends.
    TwoEmits,
}

type Input = Scene;
/// The cells of the rows found; then 99, the rows the run answered, the steps, the rows visited, the candidates and the
/// words counted as not run; then how many cursors are left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_branch_walks_a_table_reads_tests_and_emits"),
        word: Word::Succeeds,
        input: || Scene::ReadTestEmit,
        expect: Expect::Value(Cow::Borrowed(&[1, 7, 3, 7, 99, 2, 13, 3, 3, 0, 0])),
    },
    Case {
        name: CaseName("a_word_the_loop_does_not_run_ends_each_candidate_and_is_counted"),
        word: Word::Succeeds,
        input: || Scene::WithAnUnbuiltWord,
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 11, 3, 3, 2, 0])),
    },
    Case {
        name: CaseName("the_rows_a_run_answers_are_its_emits"),
        word: Word::Succeeds,
        input: || Scene::TwoEmits,
        expect: Expect::Value(Cow::Borrowed(&[
            1, 7, 1, 7, 3, 7, 3, 7, 99, 4, 15, 3, 3, 0, 0,
        ])),
    },
];

/// The scene's rule as a pack of one rule and one table.
fn pack(scene: Scene) -> LoweredPack {
    let made = StepRows::Made(MadeTableId(0));
    let read = |column: u16, out: u32| ProgramStep::Read {
        source: made,
        column: ReadColumn::Made(MadeColumn(column)),
        row: Register(0),
        out: Register(out),
    };
    let mut steps = vec![
        ProgramStep::EachRow {
            source: made,
            out: Register(0),
        },
        read(1, 1),
        ProgramStep::Test {
            test: TestOperation::Greater,
            left: StepOperand::Register(Register(1)),
            right: StepOperand::Word(CellWord(5)),
        },
        read(0, 2),
    ];
    if matches!(scene, Scene::WithAnUnbuiltWord) {
        steps.push(ProgramStep::Call {
            meaning: CallMeaning::Lower,
            arguments: StepOperands(Box::new([])),
            outputs: RegisterList(Box::new([])),
        });
    }
    let emits = if matches!(scene, Scene::TwoEmits) {
        2
    } else {
        1
    };
    for _ in 0..emits {
        steps.push(ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: StepOperands(Box::new([
                StepOperand::Register(Register(2)),
                StepOperand::File,
            ])),
        });
    }
    LoweredPack {
        definitions: Vec::new(),
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
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number, CellKind::File]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(3),
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
    let mut held = state(3);
    table(&mut held, 0, 2, &[&[1, 10], &[2, 3], &[3, 7]]);
    let (rows, found) = framed(&programs, &mut held, |frame| run_branch(frame, entry));
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.extend([
        99,
        rows.0,
        held.work.steps.0,
        held.work.visited.0,
        held.work.candidates.0,
        held.work.unbuilt.0,
        u64::try_from(held.frames.len()).unwrap_or(u64::MAX),
    ]);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
