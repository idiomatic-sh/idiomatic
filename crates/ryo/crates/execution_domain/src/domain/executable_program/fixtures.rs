//! The requests the seam D cases plan: rules written as their steps, what their parameters are bound to and the bound
//! observations they read.

use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, LiteralText, ObservationBindingId, Operand,
    ParameterId, PlanComparison, QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

/// A request of rules, each written as its steps and what its parameter is bound to.
#[derive(Clone, Copy, Debug)]
pub(super) enum Request {
    /// Two rules with the same select of a relation, bound alike.
    EqualSteps,
    /// The same select, once on each file and once after the files.
    FileAndProjectScope,
    /// The same select comparing with a parameter, bound to two values.
    DifferentParameterValues,
    /// A rule that reads a bound input in each file.
    BoundInput,
    /// A select of a column its relation does not have.
    SchemaFault,
    /// Two selects of the tree by kind, one behind a comparison that may fail.
    RootKinds,
    /// A select read by a project, both kept by the rule's frontier.
    SharedReads,
    /// Two rules with the same select in each file, each projecting it after the files from its frontier.
    TwoRulesFrontiers,
    /// Two rules with the same select after the files of a bound observation fed after the files.
    TwoRulesObservations,
    /// A select after the files of a bound observation, a projection of it, and a projection of a frontier entry.
    ObservationReach,
    /// Two rules shaped like B008 and B006, each reading one shared two-step fragment: the first after its own two steps, the
    /// second before them.
    SharedFragment,
    /// Six rules, each reading one shared four-step fragment after a different number of its own steps.
    SharedFragmentAfterOwnSteps,
}

/// A comparison of a column with an operand.
fn compared(test: Test, column: u32, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// A written text.
fn text(written: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(written.into())))
}

/// A select of `from` by `comparisons`, keeping every column.
fn select(from: StepInput, comparisons: Vec<PlanComparison>) -> Operation {
    Operation::Select {
        from,
        comparisons,
        kept: Vec::new(),
    }
}

/// A plan of `steps` that emits its last step and hands `frontier` to the steps after the files.
fn plan(steps: Vec<PlanStep>, frontier: Vec<StepInput>) -> Arc<Plan> {
    bound_plan(steps, frontier, Vec::new())
}

/// A plan of `steps` with `observation_inputs`, that emits its last step and hands `frontier` to the steps after the files.
fn bound_plan(
    steps: Vec<PlanStep>,
    frontier: Vec<StepInput>,
    observation_inputs: Vec<BoundObservationInput>,
) -> Arc<Plan> {
    Arc::new(Plan {
        observation_inputs,
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier,
        emit: Emit {
            step: StepId(u32::try_from(steps.len().saturating_sub(1)).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps,
    })
}

/// A step on each file.
fn per_file(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// A step after the files.
fn after_files(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::CrossFile,
    }
}

/// A bound observation of one file column, fed after the files.
fn fed_after() -> BoundObservationInput {
    BoundObservationInput {
        name: ObservationBindingName("files".into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        }],
        scope: StepScope::CrossFile,
    }
}

/// A projection of `from`'s first column, each row once.
fn first_column(from: StepInput) -> Operation {
    Operation::Project {
        from,
        columns: vec![ColumnIndex(0)],
        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
    }
}

/// The steps of a fragment several rules read, written from step `at` of a rule: a select of the comments' stand-in, a
/// projection of it, a select of that projection, and the union of the two.
fn fragment(at: u32) -> Vec<PlanStep> {
    vec![
        per_file(select(
            StepInput::Relation(Relation::Function),
            vec![compared(Test::Equal, 1, text("noqa"))],
        )),
        per_file(first_column(StepInput::Step(StepId(at)))),
        per_file(select(StepInput::Step(StepId(at + 1)), Vec::new())),
        per_file(Operation::Union {
            left: StepInput::Step(StepId(at + 1)),
            right: StepInput::Step(StepId(at + 2)),
        }),
    ]
}

/// A rule's own steps from step `at`: a select of the functions named `name`, then `projections` projections, each of the step
/// before it.
fn own_steps(at: u32, name: &str, projections: u32) -> Vec<PlanStep> {
    std::iter::once(per_file(select(
        StepInput::Relation(Relation::Function),
        vec![compared(Test::Equal, 1, text(name))],
    )))
    .chain(
        (at..at + projections)
            .map(|before| per_file(first_column(StepInput::Step(StepId(before))))),
    )
    .collect()
}

/// The union of a rule's steps `left` and `right`.
fn union_of(left: u32, right: u32) -> PlanStep {
    per_file(Operation::Union {
        left: StepInput::Step(StepId(left)),
        right: StepInput::Step(StepId(right)),
    })
}

/// The rules of `request`, each with what its parameter is bound to.
pub(super) fn rules_of(request: Request) -> Vec<(Arc<Plan>, Vec<ParameterBinding>)> {
    let named = || {
        select(
            StepInput::Relation(Relation::Function),
            vec![compared(Test::Equal, 1, text("a"))],
        )
    };
    let unbound = Vec::new;
    match request {
        Request::EqualSteps => vec![
            (plan(vec![per_file(named())], Vec::new()), unbound()),
            (plan(vec![per_file(named())], Vec::new()), unbound()),
        ],
        Request::FileAndProjectScope => vec![
            (plan(vec![per_file(named())], Vec::new()), unbound()),
            (
                plan(
                    vec![PlanStep {
                        operation: named(),
                        scope: StepScope::CrossFile,
                    }],
                    Vec::new(),
                ),
                unbound(),
            ),
        ],
        Request::DifferentParameterValues => {
            let parameter = || {
                select(
                    StepInput::Relation(Relation::Function),
                    vec![compared(Test::Equal, 1, Operand::Parameter(ParameterId(0)))],
                )
            };
            let bound = |value: &str| vec![ParameterBinding::Text(LiteralText(value.into()))];
            vec![
                (plan(vec![per_file(parameter())], Vec::new()), bound("a")),
                (plan(vec![per_file(parameter())], Vec::new()), bound("c")),
            ]
        }
        Request::BoundInput => vec![(
            plan(
                vec![per_file(select(
                    StepInput::BoundObservation(ObservationBindingId(0)),
                    Vec::new(),
                ))],
                Vec::new(),
            ),
            unbound(),
        )],
        Request::SchemaFault => vec![(
            plan(
                vec![per_file(select(
                    StepInput::Relation(Relation::Function),
                    vec![compared(Test::Equal, 99, text("a"))],
                ))],
                Vec::new(),
            ),
            unbound(),
        )],
        Request::RootKinds => vec![
            (
                plan(
                    vec![per_file(select(
                        StepInput::Relation(Relation::SyntaxTree),
                        vec![compared(Test::Equal, 1, text("word"))],
                    ))],
                    Vec::new(),
                ),
                unbound(),
            ),
            (
                plan(
                    vec![per_file(select(
                        StepInput::Relation(Relation::SyntaxTree),
                        vec![
                            compared(Test::Matches, 2, text("[")),
                            compared(Test::Equal, 1, text("other")),
                        ],
                    ))],
                    Vec::new(),
                ),
                unbound(),
            ),
        ],
        Request::TwoRulesFrontiers => (0..2)
            .map(|_| {
                (
                    plan(
                        vec![
                            per_file(named()),
                            after_files(first_column(StepInput::Step(StepId(0)))),
                        ],
                        vec![StepInput::Step(StepId(0))],
                    ),
                    unbound(),
                )
            })
            .collect(),
        Request::TwoRulesObservations => (0..2)
            .map(|_| {
                let bound = StepInput::BoundObservation(ObservationBindingId(0));
                (
                    bound_plan(
                        vec![after_files(select(bound, Vec::new()))],
                        vec![bound],
                        vec![fed_after()],
                    ),
                    unbound(),
                )
            })
            .collect(),
        Request::ObservationReach => {
            let bound = StepInput::BoundObservation(ObservationBindingId(0));
            vec![(
                bound_plan(
                    vec![
                        per_file(named()),
                        after_files(select(bound, Vec::new())),
                        after_files(first_column(StepInput::Step(StepId(1)))),
                        after_files(first_column(StepInput::Step(StepId(0)))),
                    ],
                    vec![bound, StepInput::Step(StepId(0))],
                    vec![fed_after()],
                ),
                unbound(),
            )]
        }
        Request::SharedFragment => {
            // B008-shaped: its own select and projection (steps 0-1), then the fragment's select and projection (steps 2-3).
            let first: Vec<PlanStep> = own_steps(0, "b008", 1)
                .into_iter()
                .chain(fragment(2).into_iter().take(2))
                .chain(std::iter::once(union_of(1, 3)))
                .collect();
            // B006-shaped: the fragment's select and projection (steps 0-1), then its own select and projection (steps 2-3).
            let second: Vec<PlanStep> = fragment(0)
                .into_iter()
                .take(2)
                .chain(own_steps(2, "b006", 1))
                .chain(std::iter::once(union_of(3, 1)))
                .collect();
            vec![
                (plan(first, Vec::new()), unbound()),
                (plan(second, Vec::new()), unbound()),
            ]
        }
        Request::SharedFragmentAfterOwnSteps => (1..=6u32)
            .map(|own| {
                let steps: Vec<PlanStep> = own_steps(0, &format!("rule{own}"), own)
                    .into_iter()
                    .chain(fragment(own + 1))
                    .chain(std::iter::once(union_of(own, own + 4)))
                    .collect();
                (plan(steps, Vec::new()), unbound())
            })
            .collect(),
        Request::SharedReads => vec![(
            plan(
                vec![
                    per_file(named()),
                    per_file(Operation::Project {
                        from: StepInput::Step(StepId(0)),
                        columns: vec![ColumnIndex(0)],
                        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
                    }),
                ],
                vec![StepInput::Step(StepId(0))],
            ),
            unbound(),
        )],
    }
}
