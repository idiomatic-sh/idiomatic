//! The programs the streaming cases run over Defines rows and the tree: a capture, selects and their comparisons, the
//! span-between stage over a faulting filter, joins and their answers, a chain of three stages, the probe programs over each
//! kind of right rows, and a probe whose one left row's answers pass the bound.
#![cfg(test)]
use super::cases::{compared, defines_of_nodes, self_matching};
use super::streams::{Program, Ran, ran_over_tree};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::ProbeMode;
use crate::domain::physical_plan::fixtures::{Step, over_defines, over_nodes};
use crate::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::{Integer, Offset, Relation, Span};
use ryo_values::domain::plan_parts::types::{
    CapturePattern, ColumnIndex, Input as StepInput, JoinColumn, LiteralText, Operand,
    PlanComparison, QueryLiteral, RepeatedRows, Side, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;

/// A select of every Defines row keeping its name and node (2 cells), an extension of it by a number, and a projection of
/// the name and the number: three stages of one chain over `rows` rows, named in turn by three names.
pub(crate) fn three_stages(rows: usize) -> Program {
    let names = ["a", "b", "ab"];
    Program {
        steps: vec![
            over_defines(Operation::Select {
                from: StepInput::Relation(Relation::Defines),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(3), ColumnIndex(0)],
            }),
            over_nodes(
                Operation::Extend {
                    from: StepInput::Step(StepId(0)),
                    values: vec![Operand::Literal(QueryLiteral::Number(Integer(7)))],
                },
                &[0],
            ),
            over_nodes(
                Operation::Project {
                    from: StepInput::Step(StepId(1)),
                    columns: vec![ColumnIndex(0), ColumnIndex(2)],
                    distinct: RepeatedRows::Kept,
                },
                &[1],
            ),
        ],
        emit: PlanNodeId(2),
        rows: (0..rows)
            .map(|at| {
                (
                    names
                        .get(at % names.len())
                        .copied()
                        .unwrap_or("a")
                        .to_owned(),
                    span(0, 1),
                )
            })
            .collect(),
        texts: names.iter().map(|text| (*text).to_owned()).collect(),
    }
}

/// 20,000 Defines rows named `x` and one named `y`. A select of the row named `y` (node 0); an inner join of it with Defines
/// on their file (node 1), so its one left row matches every row, 20,001 answers of 2 cells; and an extension of the join
/// (node 2), so the join's answers are a chunk handed on.
pub(crate) fn one_row_past_the_bound() -> Program {
    let defines = (
        StepInput::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    );
    let mut rows: Vec<(String, Span)> = (0..20_000).map(|_| ("x".to_owned(), span(0, 1))).collect();
    rows.push(("y".to_owned(), span(0, 1)));
    Program {
        steps: vec![
            over_defines(Operation::Select {
                from: defines.0,
                comparisons: vec![against(Test::Equal, 3, "y")],
                kept: Vec::new(),
            }),
            Step {
                operation: Operation::Join {
                    left: StepInput::Step(StepId(0)),
                    right: defines.0,
                    left_on: vec![ColumnIndex(5)],
                    right_on: vec![ColumnIndex(5)],
                    output: both_nodes(),
                    filter: Vec::new(),
                    view: Vec::new(),
                    fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
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
        rows,
        texts: vec!["x".into(), "y".into()],
    }
}

/// A capture of each name into the two slots of `{a}{b}`.
pub(crate) fn split_names() -> Step {
    over_defines(Operation::Capture {
        from: StepInput::Relation(Relation::Defines),
        path: ColumnIndex(3),
        pattern: CapturePattern("{a}{b}".into()),
    })
}

/// A select of every column of node `from`, kept where `comparisons` hold.
pub(crate) fn select_of(from: u32, comparisons: Vec<PlanComparison>) -> Step {
    over_nodes(
        Operation::Select {
            from: StepInput::Step(StepId(from)),
            comparisons,
            kept: Vec::new(),
        },
        &[from],
    )
}

/// Whether column `column` compares by `test` with the written text `text`.
pub(crate) fn against(test: Test, column: u32, text: &str) -> PlanComparison {
    compared(
        test,
        Operand::Column(ColumnIndex(column)),
        Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
    )
}

/// The span `start..end`.
pub(crate) fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

/// Rows of the span-between stage over a filter, `rows` of them, the stage failing on row `map_fault` (its span runs
/// backwards) and the filter on row `filter_fault` (its name is no pattern).
pub(crate) fn faulting(rows: usize, map_fault: usize, filter_fault: usize) -> Program {
    Program {
        steps: vec![
            over_defines(Operation::SpanBetween {
                from: StepInput::Relation(Relation::Defines),
                start: ColumnIndex(6),
                end: ColumnIndex(6),
            }),
            select_of(0, vec![self_matching(3)]),
        ],
        emit: PlanNodeId(1),
        rows: (0..rows)
            .map(|at| {
                (
                    if at == filter_fault { "[" } else { "a" }.to_owned(),
                    if at == map_fault {
                        span(2, 1)
                    } else {
                        span(0, 1)
                    },
                )
            })
            .collect(),
        texts: vec!["a".into(), "[".into()],
    }
}

/// An inner join of `left` with `right` on their names, answering `output`, its right rows kept where `filter` holds.
pub(crate) fn join(
    left: (StepInput, PlanInput),
    right: (StepInput, PlanInput),
    output: Vec<JoinColumn>,
    filter: Vec<PlanComparison>,
) -> Step {
    Step {
        operation: Operation::Join {
            left: left.0,
            right: right.0,
            left_on: vec![ColumnIndex(3)],
            right_on: vec![ColumnIndex(3)],
            output,
            filter,
            view: Vec::new(),
            fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
            reach: None,
        },
        inputs: vec![left.1, right.1],
    }
}

/// Every column of a join's left side, a row of Defines.
pub(crate) fn left_columns() -> Vec<JoinColumn> {
    (0..7)
        .map(|column| JoinColumn {
            side: Side::Left,
            column: ColumnIndex(column),
        })
        .collect()
}

/// The left's node and the right's node.
pub(crate) fn both_nodes() -> Vec<JoinColumn> {
    vec![
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(0),
        },
    ]
}

/// Defines' input, as an operation names it and as a node reads it.
pub(crate) fn defines() -> (StepInput, PlanInput) {
    (
        StepInput::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    )
}

/// Node `at`'s input, as an operation names it and as a node reads it.
pub(crate) fn node(at: u32) -> (StepInput, PlanInput) {
    (StepInput::Step(StepId(at)), PlanInput::Node(PlanNodeId(at)))
}

/// Every row of Defines.
pub(crate) fn every_define() -> Step {
    over_defines(Operation::Select {
        from: defines().0,
        comparisons: Vec::new(),
        kept: Vec::new(),
    })
}

/// The rows of Defines whose name compares by `test` with the written text `text`.
pub(crate) fn defines_named(test: Test, text: &str) -> Step {
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
pub(crate) fn paired(steps: Vec<Step>, rows: usize) -> Program {
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

/// The rows of Defines whose name matches the pattern it is: a name that is no pattern faults.
pub(crate) fn select_of_defines_matching() -> Step {
    over_defines(Operation::Select {
        from: defines().0,
        comparisons: vec![self_matching(3)],
        kept: Vec::new(),
    })
}

/// A select of a Defines row for each node of the wide tree (node 0) and an inner join of it with the tree's edges on their
/// parent (node 1), which walks the tree, run as planned or as its held twin.
pub(crate) fn walked_tree(twin: bool) -> Result<Ran, PortableOperatorFault> {
    let edges = (
        StepInput::Relation(Relation::SyntaxChild),
        PlanInput::Relation(Relation::SyntaxChild),
    );
    let mut children = join(node(0), edges, both_nodes(), Vec::new());
    if let Operation::Join {
        left_on, right_on, ..
    } = &mut children.operation
    {
        *left_on = vec![ColumnIndex(0)];
        *right_on = vec![ColumnIndex(0)];
    }
    ran_over_tree(
        &[every_define(), children],
        PlanNodeId(1),
        twin,
        &|resources, nodes| {
            vec![(
                Relation::Defines,
                defines_of_nodes(nodes, resources.symbol("word")),
            )]
        },
    )
}

/// Defines rows, `rows` of them paired by name, probed by `mode` (an inner join answering both nodes, or an antijoin on the
/// name) from a select of every row (node 1) into a held select of a part of them (node 0): the rows whose name starts with
/// `n` (every row) for an inner probe, those whose name ends with `1` for an anti probe.
pub(crate) fn probe_of_a_held_node(mode: ProbeMode, rows: usize) -> Program {
    let probe = match mode {
        ProbeMode::Anti => Step {
            operation: Operation::Antijoin {
                left: node(1).0,
                right: node(0).0,
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(3)],
            },
            inputs: vec![node(1).1, node(0).1],
        },
        ProbeMode::Inner(_) | ProbeMode::Semi | ProbeMode::Ordered(_) => {
            join(node(1), node(0), both_nodes(), Vec::new())
        }
    };
    let held = match mode {
        ProbeMode::Anti => defines_named(Test::EndsWith, "1"),
        ProbeMode::Inner(_) | ProbeMode::Semi | ProbeMode::Ordered(_) => {
            defines_named(Test::StartsWith, "n")
        }
    };
    paired(vec![held, every_define(), probe], rows)
}

/// 7,020 Defines rows paired by name, rows 1,000 and 1,001 named `[`. A select of every row (node 0); an inner join of it
/// with Defines on the name, every left column answered, two answers a row (node 1); a select of the join's rows whose name
/// matches itself (node 2), which faults on the answers of row 1,000, in the middle of a chunk the running probe handed on;
/// and an extension of it (node 3). The probe, before the stopped filter, reads on: it is fed every left row, and the
/// filter every answer the probe writes. (A probe's hard fault arrives only at its first left row, from its index build,
/// so a running probe cannot raise one after a later stage stopped.)
pub(crate) fn probe_before_a_stopped_filter() -> Program {
    let rows = 7_020;
    let mut names: Vec<String> = (0..rows / 2).map(|at| format!("n{at}")).collect();
    names.push("[".into());
    let defines = (
        StepInput::Relation(Relation::Defines),
        PlanInput::Relation(Relation::Defines),
    );
    let every = over_defines(Operation::Select {
        from: defines.0,
        comparisons: Vec::new(),
        kept: Vec::new(),
    });
    let probe = join(
        (StepInput::Step(StepId(0)), PlanInput::Node(PlanNodeId(0))),
        defines,
        left_columns(),
        Vec::new(),
    );
    Program {
        steps: vec![
            every,
            probe,
            select_of(1, vec![self_matching(3)]),
            over_nodes(
                Operation::Extend {
                    from: StepInput::Step(StepId(2)),
                    values: vec![Operand::Literal(QueryLiteral::Number(Integer(1)))],
                },
                &[2],
            ),
        ],
        emit: PlanNodeId(3),
        rows: (0..rows)
            .map(|at| {
                let name = if at / 2 == 500 {
                    "[".to_owned()
                } else {
                    format!("n{}", at / 2)
                };
                (name, span(0, 1))
            })
            .collect(),
        texts: names,
    }
}
