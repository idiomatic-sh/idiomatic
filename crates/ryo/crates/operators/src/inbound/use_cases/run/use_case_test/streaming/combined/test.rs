//! Streaming changes no answer: every node of the combined case programs (the case program and one node of each text kind
//! over Defines, and both ordered existence shapes over the semantic edges), each emitted alone, answers as planned what its
//! held twin answers, rows in order and the first fault alike. An ordered probe's left rows are its right side's source (the
//! rewrite proves the shape so), so its left is read twice and held, never streamed: its answer is its twin's. After a large
//! file every chain's scratch keeps no more room than the bound, and a chain's stage calls grow with its rows, not
//! with their square.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::ProbeMode;
use crate::domain::physical_plan::fixtures::ordered::{
    EDGES, OrderedShape, ordered_existence, ordered_existence_pair, ordered_rows,
};
use crate::domain::physical_plan::fixtures::{
    DEFINES, Step, defines_rows, over_defines, over_nodes, text_program,
};
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::lowering::value::{Lowering, ProbeNode};
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::stream_scratch::value::STREAM_CHUNK_CELLS;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeFlow, PlanInput, PlanInputList, PlanNodeId,
};
use crate::inbound::use_cases::run::fixtures::stream_programs::{
    against, both_nodes, span, three_stages,
};
use crate::inbound::use_cases::run::fixtures::streams::{
    Program, Steps, agrees, ran_over_tree, ran_program, ran_steps,
};
use crate::inbound::use_cases::run::fixtures::{TextResources, planned};
use ryo_relations::domain::relations::types::{Integer, Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, Operand, QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    EveryCombinedCase,
    OrderedProbe,
    LargeFile,
    TenThenFortyThousandRows,
}

/// What a case observed.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// How many nodes were compared, the ones whose planned answer differs from their twin's, and how many formed fewer
    /// node tables than their twin: the nodes a chain streamed into.
    Equivalent {
        compared: usize,
        differing: Cow<'static, str>,
        streamed: usize,
    },
    /// Whether the antijoin is an ordered probe, whether its left (the select) streams, whether it answers as its twin
    /// does, how many node tables it formed and how many rows it answered.
    Ordered {
        ordered: bool,
        streams: bool,
        same: bool,
        formed: u32,
        rows: usize,
    },
    /// Whether the large file answers as its twin, and whether every chain scratch the worker kept after it (each stage's
    /// chunk, each probe's kept right rows, each head chunk) holds no more room than the bound.
    Bounded { same: bool, bounded: bool },
    /// How the stage calls of a chain grow from 10,000 rows to 40,000.
    Growth(Growth),
}

/// How a chain's work grows from the smaller file to the larger, four times its rows.
#[derive(Debug, PartialEq, Eq)]
enum Growth {
    /// Its stage calls within four times and three.
    Linear,
    /// Past either.
    Superlinear,
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("streamed_and_held_answer_the_same_on_every_combined_case"),
        word: Word::Succeeds,
        input: || Scenario::EveryCombinedCase,
        expect: Expect::Value(Observation::Equivalent {
            compared: 38,
            differing: Cow::Borrowed(""),
            streamed: 4,
        }),
    },
    Case {
        name: CaseName("an_ordered_probes_left_is_its_right_source_and_held"),
        word: Word::Succeeds,
        input: || Scenario::OrderedProbe,
        expect: Expect::Value(Observation::Ordered {
            ordered: true,
            streams: false,
            same: true,
            formed: 2,
            rows: 4,
        }),
    },
    Case {
        name: CaseName("chain_scratch_stays_bounded_after_a_large_file"),
        word: Word::Succeeds,
        input: || Scenario::LargeFile,
        expect: Expect::Value(Observation::Bounded {
            same: true,
            bounded: true,
        }),
    },
    Case {
        name: CaseName("chained_work_grows_linearly_with_rows"),
        word: Word::Succeeds,
        input: || Scenario::TenThenFortyThousandRows,
        expect: Expect::Value(Observation::Growth(Growth::Linear)),
    },
];

/// 100,000 Defines rows named in turn by ten names. A select of every row (node 0); an inner join of it with Defines on the
/// node, keeping the right rows whose name ends with `1`, 10,000 rows (node 1); and an extension of the join (node 2).
fn large_file() -> Program {
    let names: Vec<String> = (0..10).map(|at| format!("n{at}")).collect();
    let defines = (
        StepInput::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    );
    Program {
        steps: vec![
            over_defines(Operation::Select {
                from: defines.0,
                comparisons: Vec::new(),
                kept: Vec::new(),
            }),
            Step {
                operation: Operation::Join {
                    left: StepInput::Step(StepId(0)),
                    right: defines.0,
                    left_on: vec![ColumnIndex(0)],
                    right_on: vec![ColumnIndex(0)],
                    output: both_nodes(),
                    filter: vec![against(Test::EndsWith, 3, "1")],
                    view: Vec::new(),
                    reach: None,
                },
                inputs: vec![PlanInput::Node(PlanNodeId(0)), defines.1],
            },
            over_nodes(
                Operation::Extend {
                    from: StepInput::Step(StepId(1)),
                    values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
                },
                &[1],
            ),
        ],
        emit: PlanNodeId(2),
        rows: (0..100_000)
            .map(|at| (names.get(at % 10).cloned().unwrap_or_default(), span(0, 1)))
            .collect(),
        texts: names,
    }
}

/// A select of every row of the tree (node 0) and an extension of it (node 1): a chain whose head is the tree's rows.
fn tree_chain() -> Vec<Step> {
    let tree = (
        StepInput::Relation(Relation::SyntaxTree),
        PlanInput::Relation(Relation::SyntaxTree),
    );
    vec![
        Step {
            operation: Operation::Select {
                from: tree.0,
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            inputs: vec![tree.1],
        },
        over_nodes(
            Operation::Extend {
                from: StepInput::Step(StepId(0)),
                values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
            },
            &[0],
        ),
    ]
}

/// The stage calls of the first file of the chain of three stages over `rows` rows.
fn calls(rows: usize) -> Result<usize, Failure> {
    Ok(ran_program(&three_stages(rows), false)?.stage_calls)
}

/// The file's texts: the Defines names and the semantic edges' field.
const TEXTS: &[&str] = &["alpha", "beta", "other", "gamma", "kind"];

/// The relations every combined case reads: Defines rows of the case names, and the ordered existence edges.
fn tables(resources: &TextResources) -> Vec<(Relation, Table)> {
    let named: Vec<(Cell, ryo_relations::domain::relations::types::Span)> =
        ["alpha", "beta", "other", "alpha", "gamma", "beta"]
            .into_iter()
            .map(|name| (resources.symbol(name), span(0, 1)))
            .collect();
    vec![
        (DEFINES, defines_rows(&named)),
        (EDGES, ordered_rows(resources.symbol("kind"))),
    ]
}

/// Every node of `steps`, each emitted alone, run as planned and as its held twin: the nodes compared, the ones that
/// differ (`label:node`) and the ones that formed fewer tables than their twin.
fn compared(
    label: &str,
    steps: &[Step],
    differing: &mut Vec<String>,
) -> Result<(usize, usize), Failure> {
    let mut streamed = 0;
    for at in 0..u32::try_from(steps.len()).unwrap_or(u32::MAX) {
        let run = Steps {
            steps,
            emit: PlanNodeId(at),
            first: &[],
        };
        let planned_run = ran_steps(&run, false, TEXTS, &tables)?;
        let twin = ran_steps(&run, true, TEXTS, &tables)?;
        if !agrees(&planned_run, &twin) {
            differing.push(format!("{label}:{at}"));
        }
        if planned_run.formed < twin.formed {
            streamed += 1;
        }
    }
    Ok((steps.len(), streamed))
}

/// An ordered existence over a select of every semantic edge (node 0) in place of the edges themselves: the self join
/// reads the select on both sides and the antijoin as its left, the shape at 1.
fn ordered_over_a_select() -> Vec<Step> {
    let edges = (StepInput::Relation(EDGES), PlanInput::Relation(EDGES));
    let select = (StepInput::Step(StepId(0)), PlanInput::Node(PlanNodeId(0)));
    let mut steps = vec![Step {
        operation: Operation::Select {
            from: edges.0,
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        inputs: vec![edges.1],
    }];
    for mut step in ordered_existence(
        1,
        OrderedShape {
            test: Test::LessThan,
            order: 2,
            key: &[0, 1, 2],
            shared_select: false,
        },
    ) {
        for input in &mut step.inputs {
            if *input == edges.1 {
                *input = select.1;
            }
        }
        if let Operation::Join { left, right, .. } | Operation::Antijoin { left, right, .. } =
            &mut step.operation
        {
            for side in [left, right] {
                if *side == edges.0 {
                    *side = select.0;
                }
            }
        }
        steps.push(step);
    }
    steps
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::LargeFile => {
            let program = large_file();
            let streamed = ran_program(&program, false)?;
            let twin = ran_program(&program, true)?;
            let within = |table: &Table| table.cells.capacity() <= STREAM_CHUNK_CELLS;
            // The tree's rows are decoded into the head chunk, its room kept after the run: past the bound's half, within it.
            let tree = ran_over_tree(&tree_chain(), PlanNodeId(1), false, &|_, _| Vec::new())?;
            let head_kept = tree
                .buffers
                .stream
                .levels
                .iter()
                .all(|chain| within(&chain.head))
                && tree
                    .buffers
                    .stream
                    .levels
                    .iter()
                    .any(|chain| chain.head.cells.capacity() > STREAM_CHUNK_CELLS / 2);
            Ok(Observation::Bounded {
                same: agrees(&streamed, &twin),
                bounded: head_kept
                    && !streamed.buffers.stream.levels.is_empty()
                    && streamed.buffers.stream.levels.iter().all(|chain| {
                        chain
                            .stages
                            .iter()
                            .all(|stage| within(&stage.out) && within(&stage.own.kept))
                    }),
            })
        }
        Scenario::TenThenFortyThousandRows => {
            let (small_calls, large_calls) = (calls(10_000)?, calls(40_000)?);
            Ok(Observation::Growth(
                if small_calls > 0 && large_calls <= small_calls * 4 + 3 {
                    Growth::Linear
                } else {
                    Growth::Superlinear
                },
            ))
        }
        Scenario::EveryCombinedCase => {
            let mut differing = Vec::new();
            let (texts, streamed_texts) =
                compared("text_program", &text_program(), &mut differing)?;
            let (ordered, streamed_ordered) = compared(
                "ordered_existence_pair",
                &ordered_existence_pair(),
                &mut differing,
            )?;
            Ok(Observation::Equivalent {
                compared: texts + ordered,
                differing: Cow::Owned(differing.join(" ")),
                streamed: streamed_texts + streamed_ordered,
            })
        }
        Scenario::OrderedProbe => {
            let steps = ordered_over_a_select();
            let emit = PlanNodeId(u32::try_from(steps.len()).unwrap_or(u32::MAX) - 1);
            let mut program = LogicalProgram::new();
            for step in &steps {
                let _id = program.node(LogicalNode {
                    operation: &step.operation,
                    inputs: PlanInputList(&step.inputs),
                    bindings: NodeBindings(&[]),
                    scope: StepScope::PerFile,
                });
            }
            program.emit(RuleOrdinal(0), emit);
            let plan = planned(&program)?;
            let ordered = plan.nodes.get(usize::try_from(emit.0).unwrap_or(usize::MAX)).is_some_and(
                |node| matches!(&node.lowering, Lowering::Probe(ProbeNode { probe, .. }) if matches!(probe.mode, ProbeMode::Ordered(_))),
            );
            let streams = plan
                .nodes
                .first()
                .is_some_and(|node| node.flow == NodeFlow::Streamed);
            let run = Steps {
                steps: &steps,
                emit,
                first: &[],
            };
            let planned_run = ran_steps(&run, false, TEXTS, &tables)?;
            let twin = ran_steps(&run, true, TEXTS, &tables)?;
            Ok(Observation::Ordered {
                ordered,
                streams,
                same: agrees(&planned_run, &twin),
                formed: planned_run.formed,
                rows: planned_run.answer.as_ref().map_or(0, Vec::len),
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
