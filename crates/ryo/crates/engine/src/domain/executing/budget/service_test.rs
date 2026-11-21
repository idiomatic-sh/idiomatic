//! The budget's cases, over fixture packs: a rule whose work passes its allowance is noticed and finds every row; one just
//! under is not; a rule is charged a pattern's answers alone or after another rule made them, the same; an answer reached
//! through another answer's list is charged too. The charge of fills is seen through `dispatch`, in its cases.

use super::service::{begin_rule, end_rule};
use super::types::FileSizes;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::ShapeChoice;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::executing::service::run as run_program;
use crate::domain::findings::types::{ColumnKinds, HeaviestWork, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill,
    MadeTable, PackDefinition, PatternForm,
};
use crate::domain::programs::fixtures::{framed, held, table};
use crate::domain::programs::frames::types::RunMode;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, PlacedStep, Polarity,
    ProgramStep, ReadColumn, Register, RegisterCount, RegisterList, StepBody, StepMemo,
    StepOperand, StepOperands, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// What a case loads: how many rules, which definition each asks for every row of the table (0 the heavy one, 1 the one
/// that only asks the heavy one, 2 the heavy one after an inline body of the rule's own has run), how many rows each of the heavy pattern's two nested `Or` words gives, and the keys the
/// table's rows hold.
type Input = (u32, u32, u64, &'static [u64]);
/// For each notice the rule, the work, the allowance, the heaviest unit's kind (0 the rule's own program, 1 a called
/// pattern), its keys or candidates and its rows visited; then 99 and how many rows were found.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_whose_work_passes_its_allowance_is_noticed_and_finds_every_row"),
        word: Word::Succeeds,
        input: || (1, 0, 12, &[1, 2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[0, 471, 384, 1, 3, 468, 99, 3])),
    },
    Case {
        name: CaseName("a_rule_just_under_its_allowance_is_not_noticed"),
        word: Word::Succeeds,
        input: || (1, 0, 10, &[1, 2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[99, 3])),
    },
    Case {
        name: CaseName("alone_or_after_another_rule_the_same_notice"),
        word: Word::Succeeds,
        input: || (2, 0, 12, &[1, 2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[
            0, 471, 384, 1, 3, 468, 1, 471, 384, 1, 3, 468, 99, 6,
        ])),
    },
    Case {
        name: CaseName("an_answer_reached_through_another_answers_list_is_charged_to_each_rule"),
        word: Word::Succeeds,
        input: || (2, 1, 14, &[1, 2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[
            0, 633, 576, 1, 3, 630, 1, 633, 576, 1, 3, 630, 99, 6,
        ])),
    },
    Case {
        name: CaseName("a_key_asked_twice_is_charged_once"),
        word: Word::Succeeds,
        input: || (1, 0, 12, &[1, 1, 2]),
        expect: Expect::Value(Cow::Borrowed(&[99, 3])),
    },
    Case {
        name: CaseName("a_patterns_work_is_its_own_not_what_its_caller_did_before"),
        word: Word::Succeeds,
        input: || (1, 2, 12, &[1, 2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[0, 474, 384, 1, 3, 468, 99, 3])),
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

/// An `Or` of `rows` inline bodies, each yielding one constant, into one register.
fn or_of(rows: u64, out: u32) -> ProgramStep {
    ProgramStep::Or {
        bodies: (0..rows)
            .map(|number| {
                StepBody::Inline(program(vec![
                    ProgramStep::Let {
                        from: StepOperand::Word(CellWord(number)),
                        out: Register(3),
                    },
                    ProgramStep::Yield {
                        row: RegisterList(Box::new([Register(3)])),
                    },
                ]))
            })
            .collect(),
        outputs: RegisterList(Box::new([Register(out)])),
        memo: StepMemo::None,
    }
}

/// A use of a definition's pattern that binds its one column to a register and collects nothing.
fn asks(definition: u32, value: u32) -> ProgramStep {
    ProgramStep::Exists {
        polarity: Polarity::Present,
        body: StepBody::Definition {
            definition: DefinitionId(definition),
            bound: ColumnMask(Box::new([MadeColumn(0)])),
            collected: ColumnMask(Box::new([])),
            values: StepOperands(Box::new([StepOperand::Register(Register(value))])),
        },
        memo: StepMemo::None,
    }
}

/// A definition of one column and one pattern in call form, which binds the column and collects nothing.
fn definition(node: u8, formal: u32, steps: Vec<ProgramStep>) -> PackDefinition {
    let mut body = steps;
    body.push(ProgramStep::Yield {
        row: RegisterList(Box::new([])),
    });
    PackDefinition {
        node: NodeDigest([node; 16]),
        formals: RegisterList(Box::new([Register(formal)])),
        emptiness: Emptiness::Unknown,
        patterns: vec![DefinitionPattern {
            bound: ColumnMask(Box::new([MadeColumn(0)])),
            collected: ColumnMask(Box::new([])),
            shape: ShapeChoice::Decided,
            form: PatternForm::Call(program(body)),
        }],
    }
}

/// A rule that asks `asked` for every row of the table and emits the row's key.
fn rule(number: u32, asked: u32) -> LoweredRule {
    let source = StepRows::Made(MadeTableId(0));
    LoweredRule {
        rule: SelectedRuleOrdinal(number),
        node: NodeDigest([u8::try_from(number).unwrap_or(u8::MAX).wrapping_add(20); 16]),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: program(vec![
                ProgramStep::EachRow {
                    source,
                    out: Register(0),
                },
                ProgramStep::Read {
                    source,
                    column: ReadColumn::Made(MadeColumn(0)),
                    row: Register(0),
                    out: Register(1),
                },
                ProgramStep::Exists {
                    polarity: Polarity::Present,
                    body: StepBody::Inline(program(if asked == 2 {
                        vec![or_of(2, 7)]
                    } else {
                        Vec::new()
                    })),
                    memo: StepMemo::None,
                },
                asks(asked % 2, 1),
                ProgramStep::Emit {
                    target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
                    columns: StepOperands(Box::new([StepOperand::Register(Register(1))])),
                },
            ]),
        }],
        columns: RuleColumns {
            emit: ColumnKinds(vec![CellKind::Number]),
            frontier: Vec::new(),
        },
    }
}

fn subject((rules, asked, rows, keys): Input) -> Result<Value, Failure> {
    let pack = LoweredPack {
        definitions: vec![
            definition(3, 5, vec![or_of(rows, 7), or_of(rows, 8)]),
            definition(4, 9, vec![asks(0, 9)]),
        ],
        tables: vec![MadeTable {
            node: NodeDigest([2; 16]),
            fill: MadeFill::Constant(ConstantRows(Vec::new())),
            width: ColumnCount(1),
            indexes: Vec::new(),
            emptiness: Emptiness::Unknown,
        }],
        rules: (0..rules).map(|number| rule(number, asked)).collect(),
        registers: RegisterCount(10),
        regexes: Vec::new(),
        texts: Vec::new(),
        reads: Vec::new(),
    };
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let mut held = held(10);
    let held_rows: Vec<&[u64]> = keys.chunks(1).collect();
    table(&mut held, 0, 1, &held_rows);
    let (notices, found) = framed(&programs.0, &mut held, &TheLoop, |frame| {
        let mut notices = Vec::new();
        for each in &programs.0.code.rules {
            begin_rule(frame.state);
            for branch in &each.branches {
                let _rows = run_program(frame, branch.entry, RunMode::Rows);
            }
            let sizes = FileSizes {
                store: frame.fill.store,
                nodes: frame.fill.nodes,
            };
            notices.extend(end_rule(frame.state, sizes, &programs, each));
        }
        notices
    });
    let mut all = Vec::new();
    for notice in notices {
        let (kind, counted, visited) = match notice.heaviest {
            HeaviestWork::Program {
                candidates,
                visited,
            } => (0, candidates, visited),
            HeaviestWork::Calls { keys, visited } => (1, keys, visited),
            _ => return Err(()),
        };
        all.extend([
            u64::from(notice.rule.0),
            notice.work.0,
            notice.allowance.0,
            kind,
            counted.0,
            visited.0,
        ]);
    }
    all.extend([99, u64::try_from(found.rows.len()).unwrap_or(u64::MAX)]);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
