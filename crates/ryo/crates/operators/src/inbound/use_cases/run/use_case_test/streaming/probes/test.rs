//! A probe's left rows stream into it, each against its held twin: an inner, a semi and an anti probe behind a select, the
//! right side's fault raised before any left row, a right filter kept across every chunk of left rows, and two probes of
//! one chain each reading its own index while both are active.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, over_defines};
use crate::domain::physical_plan::types::stream_scratch::value::STREAM_CHUNK_CELLS;
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use crate::inbound::use_cases::run::fixtures::cases::{compared, self_matching};
use crate::inbound::use_cases::run::fixtures::streams::{
    Program, agrees, both_nodes, join, left_columns, ran_after, span,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, LiteralText, Operand, QueryLiteral, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use std::borrow::Cow;

/// What a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    Inner,
    Semi,
    AntiOfEmptyRight,
    RightFaultFirst,
    RightFilter,
    TwoIndexes,
    RightPendingFed,
    RightPendingFedAfterAnotherProbe,
}

/// Whether the program run as planned answered as its held twin did, how many rows it answered, how many node tables it
/// formed, and its fault. The row count holds a probe's own state to its answer, which a twin sharing the probe's kernel
/// cannot.
#[derive(Debug, PartialEq, Eq)]
struct Observation {
    /// Whether the two runs answered the same.
    same: bool,
    /// How many rows the planned run answered.
    rows: usize,
    /// How many node tables the planned run formed.
    formed: u32,
    /// The planned run's fault; none when it answered rows.
    fault: Cow<'static, str>,
    /// The rows the planned run's emit was fed from each input.
    fed: [usize; 3],
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_probe_left_pipeline_streams_into_it"),
        word: Word::Succeeds,
        input: || Scenario::Inner,
        expect: Expect::Value(Observation {
            same: true,
            rows: 14_040,
            formed: 1,
            fault: Cow::Borrowed(""),
            fed: [7_020, 7_020, 0],
        }),
    },
    Case {
        name: CaseName("a_probe_left_pipeline_streams_into_a_semi_probe_dropping_repeats"),
        word: Word::Succeeds,
        input: || Scenario::Semi,
        expect: Expect::Value(Observation {
            same: true,
            rows: 7_020,
            formed: 1,
            fault: Cow::Borrowed(""),
            fed: [7_020, 7_020, 0],
        }),
    },
    Case {
        name: CaseName("a_probe_left_pipeline_streams_into_an_anti_probe_of_an_empty_right"),
        word: Word::Succeeds,
        input: || Scenario::AntiOfEmptyRight,
        expect: Expect::Value(Observation {
            same: true,
            rows: 7_020,
            formed: 2,
            fault: Cow::Borrowed(""),
            fed: [7_020, 0, 0],
        }),
    },
    Case {
        name: CaseName("a_probe_right_fault_is_raised_before_any_left_row"),
        word: Word::Succeeds,
        input: || Scenario::RightFaultFirst,
        expect: Expect::Value(Observation {
            same: true,
            rows: 0,
            formed: 2,
            fault: Cow::Borrowed(
                "Pattern(InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" })",
            ),
            fed: [3, 0, 0],
        }),
    },
    Case {
        name: CaseName("a_probe_keeps_its_filtered_right_rows_across_every_chunk"),
        word: Word::Succeeds,
        input: || Scenario::RightFilter,
        expect: Expect::Value(Observation {
            same: true,
            rows: 1_404,
            formed: 1,
            fault: Cow::Borrowed(""),
            fed: [7_020, 7_020, 0],
        }),
    },
    Case {
        name: CaseName("two_probes_of_one_chain_each_read_their_own_index"),
        word: Word::Succeeds,
        input: || Scenario::TwoIndexes,
        expect: Expect::Value(Observation {
            same: true,
            rows: 4_680,
            formed: 1,
            fault: Cow::Borrowed(""),
            fed: [9_360, 4_680, 0],
        }),
    },
    Case {
        name: CaseName("a_probe_of_a_faulting_right_is_fed_its_left_and_its_right_rows"),
        word: Word::Succeeds,
        input: || Scenario::RightPendingFed,
        expect: Expect::Value(Observation {
            same: true,
            rows: 0,
            formed: 2,
            fault: Cow::Borrowed(
                "Pattern(InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" })",
            ),
            fed: [8, 4, 0],
        }),
    },
    Case {
        name: CaseName("a_probe_of_a_faulting_right_is_fed_its_own_rows_after_another_probe"),
        word: Word::Succeeds,
        input: || Scenario::RightPendingFedAfterAnotherProbe,
        expect: Expect::Value(Observation {
            same: true,
            rows: 0,
            formed: 4,
            fault: Cow::Borrowed(
                "Pattern(InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" })",
            ),
            fed: [8, 4, 0],
        }),
    },
];

/// Defines' input, as an operation names it and as a node reads it.
fn defines() -> (StepInput, PlanInput) {
    (
        StepInput::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    )
}

/// Node `at`'s input, as an operation names it and as a node reads it.
fn node(at: u32) -> (StepInput, PlanInput) {
    (StepInput::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)))
}

/// Every row of Defines.
fn every_define() -> Step {
    over_defines(Operation::Select {
        from: defines().0,
        comparisons: Vec::new(),
        kept: Vec::new(),
    })
}

/// The rows of Defines whose name compares by `test` with the written text `text`.
fn defines_named(test: Test, text: &str) -> Step {
    over_defines(Operation::Select {
        from: defines().0,
        comparisons: vec![compared(
            test,
            Operand::Column(ColumnIndex(3)),
            Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        )],
        kept: Vec::new(),
    })
}

/// Defines rows, `rows` of them over three chunks of left rows and more, each name `n<k>` shared by the two rows `2k` and
/// `2k + 1`, so each left row's name has two right rows.
fn paired(steps: Vec<Step>, rows: usize) -> Program {
    let names: Vec<String> = (0..rows.div_ceil(2)).map(|at| format!("n{at}")).collect();
    Program {
        emit: PlanNodeId(
            u32::try_from(steps.len())
                .unwrap_or(u32::MAX)
                .saturating_sub(1),
        ),
        steps,
        rows: (0..rows)
            .map(|at| (names.get(at / 2).cloned().unwrap_or_default(), span(0, 1)))
            .collect(),
        texts: names,
    }
}

/// The program each scenario runs.
fn program(scenario: Scenario) -> Program {
    // Rows of 7 cells: a chunk of a stage that writes Defines rows holds this many and a few more.
    let chunk = STREAM_CHUNK_CELLS / 7;
    match scenario {
        Scenario::Inner => paired(
            vec![
                every_define(),
                join(node(0), defines(), both_nodes(), Vec::new()),
            ],
            chunk * 3,
        ),
        Scenario::Semi => paired(
            vec![
                every_define(),
                Step {
                    operation: Operation::Intersection {
                        left: node(0).0,
                        right: defines().0,
                    },
                    inputs: vec![node(0).1, defines().1],
                },
            ],
            chunk * 3,
        ),
        Scenario::AntiOfEmptyRight => paired(
            vec![
                defines_named(Test::Equal, "absent"),
                every_define(),
                Step {
                    operation: Operation::Antijoin {
                        left: node(1).0,
                        right: node(0).0,
                        left_on: vec![ColumnIndex(3)],
                        right_on: vec![ColumnIndex(3)],
                    },
                    inputs: vec![node(1).1, node(0).1],
                },
            ],
            chunk * 3,
        ),
        Scenario::RightFaultFirst => {
            // The right side faults on its first row (its name is no pattern); the left's span-between stage on its fourth
            // (its span runs backwards).
            let mut program = paired(
                vec![
                    select_of_defines_matching(),
                    over_defines(Operation::SpanBetween {
                        from: defines().0,
                        start: ColumnIndex(6),
                        end: ColumnIndex(6),
                    }),
                    join(node(1), node(0), both_nodes(), Vec::new()),
                ],
                8,
            );
            if let Some(first) = program.rows.first_mut() {
                first.0 = "[".into();
            }
            if let Some(fourth) = program.rows.get_mut(3) {
                fourth.1 = span(2, 1);
            }
            program.texts.push("[".into());
            program
        }
        Scenario::RightFilter => paired(
            vec![
                every_define(),
                join(
                    node(0),
                    defines(),
                    both_nodes(),
                    vec![compared(
                        Test::EndsWith,
                        Operand::Column(ColumnIndex(3)),
                        Operand::Literal(QueryLiteral::Text(LiteralText("1".into()))),
                    )],
                ),
            ],
            chunk * 3,
        ),
        Scenario::RightPendingFed | Scenario::RightPendingFedAfterAnotherProbe => {
            // The review's input: eight Defines rows, the fifth named `[`, so the right select faults after four rows; the
            // join reads every row on its left. After another probe, a join of the left with Defines runs first in the
            // same worker.
            let mut steps = vec![
                select_of_defines_matching(),
                every_define(),
                join(node(1), node(0), both_nodes(), Vec::new()),
            ];
            if let Scenario::RightPendingFedAfterAnotherProbe = scenario {
                steps.push(join(node(1), defines(), both_nodes(), Vec::new()));
            }
            let mut program = paired(steps, 8);
            program.emit = PlanNodeId(2);
            if let Some(fifth) = program.rows.get_mut(4) {
                fifth.0 = "[".into();
            }
            program.texts.push("[".into());
            program
        }
        Scenario::TwoIndexes => paired(
            vec![
                every_define(),
                join(node(0), defines(), left_columns(), Vec::new()),
                Step {
                    operation: Operation::Intersection {
                        left: node(1).0,
                        right: defines().0,
                    },
                    inputs: vec![node(1).1, defines().1],
                },
            ],
            chunk * 2,
        ),
    }
}

/// The rows of Defines whose name matches the pattern it is: a name that is no pattern faults.
fn select_of_defines_matching() -> Step {
    over_defines(Operation::Select {
        from: defines().0,
        comparisons: vec![self_matching(3)],
        kept: Vec::new(),
    })
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let program = program(scenario);
    let first: &[PlanNodeId] = if let Scenario::RightPendingFedAfterAnotherProbe = scenario {
        &[PlanNodeId(3)]
    } else {
        &[]
    };
    let streamed = ran_after(&program, false, first)?;
    let twin = ran_after(&program, true, first)?;
    Ok(Observation {
        same: agrees(&streamed, &twin),
        rows: streamed.answer.as_ref().map_or(0, Vec::len),
        formed: streamed.formed,
        fed: streamed
            .fed
            .get(usize::try_from(program.emit.0).unwrap_or(usize::MAX))
            .copied()
            .flatten()
            .unwrap_or_default(),
        fault: Cow::Owned(
            streamed
                .answer
                .err()
                .map(|fault| format!("{fault:?}"))
                .unwrap_or_default(),
        ),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
