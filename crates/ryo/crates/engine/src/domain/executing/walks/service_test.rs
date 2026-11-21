//! The walks' cases, through the loop, over fixture packs written as steps and encoded by the straight encoding, on a
//! file's tree: a root (element 0) with three children (1, 2, 3), each holding one token (4, 5, 6), and one embedded
//! element (7) under the third child, past the file's own tree. A handle is the file (7) in its high half and the element in its low half; a
//! case shows the low half.

use super::service::{axis, nearest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::Op;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::executing::service::run as run_branch;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
};
use crate::domain::made_tables::fixtures::packs::nodes;
use crate::domain::programs::fixtures::{FILE, framed_over, held};
use crate::domain::programs::frames::types::RunMode;
use crate::domain::programs::operands::service::set;
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{
    AxisDirection, EmitTarget, MadeColumn, MadeTableId, NearestEdges, PlacedStep, ProgramStep,
    ReadColumn, Register, RegisterCount, RegisterList, StepBody, StepMemo, StepOperand,
    StepOperands, StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// What a case's rule does with each start of its table of starts.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// Emits the start and each element an axis reaches from it.
    Along(AxisDirection),
    /// Emits the start and its nearest ancestor that is not element 3; the answer kept per start or not.
    NearestNotThree(Kept),
}

/// Whether a `Nearest` keeps its answer per start.
#[derive(Clone, Copy, Debug)]
enum Kept {
    /// It walks each time.
    No,
    /// It walks once per start.
    PerStart,
}

/// The scene, and the elements the table of starts holds, in order.
type Input = (Scene, &'static [u64]);
/// The start and the element of each row found, as elements; then 99, the rows the run answered, the steps, the rows
/// visited, the words counted as not run, and how many cursors and staged cells are left; then, the word asked once
/// more for the first start, the rows it answered and the cells it left staged.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_parent_axis_reaches_one_element_and_none_from_the_root"),
        word: Word::Succeeds,
        input: || (Scene::Along(AxisDirection::Parent), &[4, 0, 3]),
        expect: Expect::Value(Cow::Borrowed(&[4, 1, 3, 0, 99, 2, 11, 5, 0, 0, 0, 1, 1])),
    },
    Case {
        name: CaseName("the_ancestor_axis_reaches_each_ancestor_nearest_first"),
        word: Word::Succeeds,
        input: || (Scene::Along(AxisDirection::Ancestor), &[4, 1]),
        expect: Expect::Value(Cow::Borrowed(&[
            4, 1, 4, 0, 1, 0, 99, 3, 11, 5, 0, 0, 0, 2, 2,
        ])),
    },
    Case {
        name: CaseName("the_descendant_axis_reaches_the_own_tree_in_pre_order"),
        word: Word::Succeeds,
        input: || (Scene::Along(AxisDirection::Descendant), &[3, 0]),
        expect: Expect::Value(Cow::Borrowed(&[
            0, 1, 0, 2, 0, 3, 99, 3, 11, 5, 0, 0, 0, 0, 0,
        ])),
    },
    Case {
        name: CaseName("the_next_sibling_axis_reaches_the_child_after"),
        word: Word::Succeeds,
        input: || (Scene::Along(AxisDirection::NextSibling), &[1, 2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 2, 3, 99, 2, 11, 5, 0, 0, 0, 1, 1])),
    },
    Case {
        name: CaseName("the_previous_sibling_axis_reaches_the_child_before"),
        word: Word::Succeeds,
        input: || (Scene::Along(AxisDirection::PreviousSibling), &[1, 2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[2, 1, 3, 2, 99, 2, 11, 5, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("nearest_is_the_first_ancestor_its_condition_holds_of_never_the_start"),
        word: Word::Succeeds,
        input: || (Scene::NearestNotThree(Kept::No), &[6, 4, 3, 0, 6]),
        expect: Expect::Value(Cow::Borrowed(&[
            6, 0, 4, 1, 3, 0, 6, 0, 99, 4, 29, 11, 0, 0, 0, 1, 1,
        ])),
    },
    Case {
        name: CaseName("a_nearest_kept_per_start_walks_once_for_a_start_asked_twice"),
        word: Word::Succeeds,
        input: || (Scene::NearestNotThree(Kept::PerStart), &[6, 4, 3, 0, 6]),
        expect: Expect::Value(Cow::Borrowed(&[
            6, 0, 4, 1, 3, 0, 6, 0, 99, 4, 26, 9, 0, 0, 0, 1, 1,
        ])),
    },
];

/// The handle of an element of the fixture file.
fn handle(element: u64) -> CellWord {
    CellWord((u64::from(FILE.0) << u32::BITS) | element)
}

/// The rule set: one rule over the table of starts.
fn pack((scene, starts): Input) -> LoweredPack {
    let made = StepRows::Made(MadeTableId(0));
    let mut steps = vec![
        ProgramStep::EachRow {
            source: made,
            out: Register(0),
        },
        ProgramStep::Read {
            source: made,
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(0),
            out: Register(1),
        },
    ];
    steps.push(match scene {
        Scene::Along(direction) => ProgramStep::Axis {
            relation: Relation::SyntaxTree,
            direction,
            start: Register(1),
            out: Register(2),
        },
        Scene::NearestNotThree(kept) => ProgramStep::Nearest {
            edges: NearestEdges::Tree(Relation::SyntaxTree),
            condition: StepBody::Inline(StepProgram(Box::new([
                PlacedStep {
                    point: None,
                    step: ProgramStep::Test {
                        test: TestOperation::NotEqual,
                        left: StepOperand::Register(Register(3)),
                        right: StepOperand::Word(handle(3)),
                    },
                },
                PlacedStep {
                    point: None,
                    step: ProgramStep::Yield {
                        row: RegisterList(Box::new([])),
                    },
                },
            ]))),
            element: Register(3),
            start: Register(1),
            out: Register(2),
            memo: match kept {
                Kept::No => StepMemo::None,
                Kept::PerStart => StepMemo::Keyed(RegisterList(Box::new([Register(1)]))),
            },
        },
    });
    steps.push(ProgramStep::Emit {
        target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
        columns: StepOperands(Box::new([
            StepOperand::Register(Register(1)),
            StepOperand::Register(Register(2)),
        ])),
    });
    LoweredPack {
        definitions: Vec::new(),
        tables: vec![MadeTable {
            node: NodeDigest([2; 16]),
            fill: MadeFill::Constant(ConstantRows(
                starts
                    .iter()
                    .map(|element| -> Box<[CellWord]> { Box::new([handle(*element)]) })
                    .collect(),
            )),
            width: ColumnCount(1),
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
                emit: ColumnKinds(vec![CellKind::Number, CellKind::Number]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(4),
        regexes: Vec::new(),
        texts: Vec::new(),
        reads: Vec::new(),
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let code = encoded(&pack(input)).map_err(|_| ())?;
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
    let (tree, mut held) = (nodes(), held(4));
    let (rows, found) = framed_over(&tree, &programs, &mut held, &TheLoop, |frame| {
        run_branch(frame, entry, RunMode::Rows)
    });
    let mut all: Vec<u64> = found
        .rows
        .into_iter()
        .flatten()
        .map(|cell| cell & u64::from(u32::MAX))
        .collect();
    all.extend([
        99,
        rows.0,
        held.state.work.steps.0,
        held.state.work.visited.0,
        held.state.work.unbuilt.0,
        u64::try_from(held.state.frames.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX),
    ]);
    // Then the word asked once more, outside a run, for the first start: what it leaves staged for its item.
    let first = input.1.first().copied().unwrap_or(0);
    let (asked, _found) = framed_over(&tree, &programs, &mut held, &TheLoop, |frame| {
        let walking = frame
            .fill
            .programs
            .code
            .words
            .iter()
            .copied()
            .find(|word| word.op == Op::Axis || word.op == Op::Nearest);
        set(frame, Register(1), handle(first));
        if let Some(word) = walking {
            if word.op == Op::Axis {
                axis(frame, word);
            } else {
                nearest(frame, word);
            }
        }
        [
            u64::try_from(frame.state.answer.rows.0).unwrap_or(u64::MAX),
            u64::try_from(frame.state.staged.len()).unwrap_or(u64::MAX),
        ]
    });
    all.extend(asked);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
