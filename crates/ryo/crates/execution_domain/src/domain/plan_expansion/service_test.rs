//! The `plan_expansion` service's cases: the one shape each expanded kind is written as, and that the plan around an
//! expansion names the same rows after it. That this engine's planner still recognises the shapes is `lowering_test`.
use super::service::{LADDER_LINKS, expanded};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::observations::types::{
    ObservationColumn, ObservationColumnName, ObservationName, PlanObservation,
};
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, HostedNumber, Input as Source, JoinColumn, Side, StepId, StepScope,
};
use ryo_values::domain::plans::service::{inputs_of, kind_of, named};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::{Emit, Plan};
use std::borrow::Cow;

#[cfg(test)]
mod lowering_test;

/// What a row asks of the expansion of one plan.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The kinds of the expanded plan's steps, in order.
    Kinds(fn() -> Plan),
    /// The tests the expanded plan's selects make, in order.
    Tests(fn() -> Plan),
    /// Whether the links of the expanded plan's ladder are one join of the step before with itself, repeated.
    Ladder(fn() -> Plan),
    /// What the expanded plan emits, hands over and exports, and what its last step reads.
    Names(fn() -> Plan),
    /// Whether the expanded plan is the plan.
    Unchanged(fn() -> Plan),
}
type Input = Ask;
type Value = Cow<'static, str>;
type Failure = std::convert::Infallible;

/// `binding-event`: its ordinal is a number, its file its sixth column.
const EVENTS: Source = Source::Relation(Relation::BindingEvent);
const ORDINAL: ColumnIndex = ColumnIndex(5);
const FILE: ColumnIndex = ColumnIndex(6);

/// A plan of `operations`, each reading the steps before it by number, all per file; it emits its last step.
fn plan_of(operations: Vec<Operation>) -> Plan {
    let last = StepId(u32::try_from(operations.len().saturating_sub(1)).unwrap_or(u32::MAX));
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps: operations
            .into_iter()
            .map(|operation| PlanStep {
                operation,
                scope: StepScope::PerFile,
            })
            .collect(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: last,
            subject: FILE,
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    }
}

fn group_bound(end: GroupEnd) -> Plan {
    plan_of(vec![Operation::GroupBound {
        from: EVENTS,
        keys: vec![FILE],
        order: ORDINAL,
        end,
    }])
}

fn as_of(bound: OrderBound) -> Plan {
    plan_of(vec![Operation::AsOf {
        left: EVENTS,
        right: EVENTS,
        left_on: vec![FILE],
        right_on: vec![FILE],
        left_order: ORDINAL,
        right_order: ORDINAL,
        bound,
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: FILE,
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ORDINAL,
            },
        ],
    }])
}

fn group_ordinal() -> Plan {
    plan_of(vec![Operation::GroupOrdinal {
        from: EVENTS,
        keys: vec![FILE],
        order: ORDINAL,
    }])
}

fn follow() -> Plan {
    plan_of(vec![Operation::Follow {
        edges: EVENTS,
        keys: vec![FILE],
        source: ColumnIndex(2),
        destination: ColumnIndex(3),
    }])
}

/// A select, a `group-max` of it, and a select of that: the last is emitted, the `group-max` is handed over and exported.
fn renumbered() -> Plan {
    let all = |from| Operation::Select {
        from,
        comparisons: Vec::new(),
        kept: Vec::new(),
    };
    let mut plan = plan_of(vec![
        all(EVENTS),
        Operation::GroupBound {
            from: Source::Step(StepId(0)),
            keys: vec![FILE],
            order: ORDINAL,
            end: GroupEnd::Greatest,
        },
        all(Source::Step(StepId(1))),
    ]);
    plan.frontier = vec![EVENTS, Source::Step(StepId(1))];
    plan.observations = vec![PlanObservation {
        name: ObservationName("latest".into()),
        source: Source::Step(StepId(1)),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        }],
        scope: StepScope::PerFile,
    }];
    plan
}

/// A `group-max` of the host's rows, whose width a plan does not say.
fn over_hosted_rows() -> Plan {
    plan_of(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::GroupBound {
            from: Source::Step(StepId(0)),
            keys: Vec::new(),
            order: ColumnIndex(0),
            end: GroupEnd::Greatest,
        },
    ])
}

/// A plan that holds no expanded kind.
fn plain() -> Plan {
    plan_of(vec![Operation::Select {
        from: EVENTS,
        comparisons: Vec::new(),
        kept: Vec::new(),
    }])
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_plan_with_no_expanded_kind_is_answered_as_it_is"),
        word: Word::Succeeds,
        input: || Ask::Unchanged(plain),
        expect: Expect::Value(Cow::Borrowed("unchanged")),
    },
    Case {
        name: CaseName(
            "group_max_is_a_self_join_a_strict_comparison_the_passed_rows_and_an_antijoin",
        ),
        word: Word::Succeeds,
        input: || Ask::Kinds(|| group_bound(GroupEnd::Greatest)),
        expect: Expect::Value(Cow::Borrowed("Join Select Project Antijoin")),
    },
    Case {
        name: CaseName("as_of_is_one_band_shape"),
        word: Word::Succeeds,
        input: || Ask::Kinds(|| as_of(OrderBound::Before)),
        expect: Expect::Value(Cow::Borrowed(
            "Join Select Join Select Project Antijoin Project",
        )),
    },
    Case {
        name: CaseName("as_of_before_bands_strictly_below_and_keeps_the_greatest"),
        word: Word::Succeeds,
        input: || Ask::Tests(|| as_of(OrderBound::Before)),
        expect: Expect::Value(Cow::Borrowed("LessThan LessThan")),
    },
    Case {
        name: CaseName("as_of_at_or_before_bands_at_or_below_and_keeps_the_greatest"),
        word: Word::Succeeds,
        input: || Ask::Tests(|| as_of(OrderBound::AtOrBefore)),
        expect: Expect::Value(Cow::Borrowed("LessThanOrEqual LessThan")),
    },
    Case {
        name: CaseName("as_of_after_bands_strictly_above_and_keeps_the_least"),
        word: Word::Succeeds,
        input: || Ask::Tests(|| as_of(OrderBound::After)),
        expect: Expect::Value(Cow::Borrowed("GreaterThan GreaterThan")),
    },
    Case {
        name: CaseName("as_of_at_or_after_bands_at_or_above_and_keeps_the_least"),
        word: Word::Succeeds,
        input: || Ask::Tests(|| as_of(OrderBound::AtOrAfter)),
        expect: Expect::Value(Cow::Borrowed("GreaterThanOrEqual GreaterThan")),
    },
    Case {
        name: CaseName("group_ordinal_counts_the_pairs_of_distinct_rows_and_gives_the_first_zero"),
        word: Word::Succeeds,
        input: || Ask::Kinds(group_ordinal),
        expect: Expect::Value(Cow::Borrowed(
            "Project Join Select GroupCount Antijoin Extend Union",
        )),
    },
    Case {
        name: CaseName("follow_is_the_distinct_pointers_and_a_ladder_of_twenty_links"),
        word: Word::Succeeds,
        input: || Ask::Ladder(follow),
        expect: Expect::Value(Cow::Borrowed(
            "22 steps; 20 links, each one join of the step before with itself, unfiltered",
        )),
    },
    Case {
        name: CaseName("emit_frontier_and_observations_name_the_same_rows_after_an_expansion"),
        word: Word::Succeeds,
        input: || Ask::Names(renumbered),
        expect: Expect::Value(Cow::Borrowed(
            "emit 5; frontier [Relation(BindingEvent), Step(StepId(4))]; observation Step(StepId(4)); last reads [Step(StepId(4))]",
        )),
    },
    Case {
        name: CaseName("an_operation_over_the_hosts_rows_is_left_as_written"),
        word: Word::Succeeds,
        input: || Ask::Kinds(over_hosted_rows),
        expect: Expect::Value(Cow::Borrowed("Hosted GroupBound")),
    },
];

/// The kinds of the steps of `plan`, in order.
fn kinds(plan: &Plan) -> String {
    plan.steps
        .iter()
        .map(|step| format!("{:?}", kind_of(&step.operation)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn subject(ask: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match ask {
        Ask::Unchanged(plan) => {
            if expanded(&plan()) == plan() {
                "unchanged".to_owned()
            } else {
                "changed".to_owned()
            }
        }
        Ask::Kinds(plan) => kinds(&expanded(&plan())),
        Ask::Tests(plan) => expanded(&plan())
            .steps
            .iter()
            .filter_map(|step| match &step.operation {
                Operation::Select { comparisons, .. } => {
                    comparisons.first().map(|made| format!("{:?}", made.test))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        Ask::Ladder(plan) => {
            let plan = expanded(&plan());
            let links: Vec<_> = plan
                .steps
                .iter()
                .enumerate()
                .filter(|(_, step)| matches!(step.operation, Operation::Join { .. }))
                .collect();
            let first = links.first().map(|(_, step)| &step.operation);
            let alike = links.iter().all(|(at, step)| {
                let before = Source::Step(StepId(u32::try_from(at.saturating_sub(1)).unwrap_or(0)));
                let (
                    Operation::Join {
                        left,
                        right,
                        left_on,
                        right_on,
                        output,
                        filter,
                        view,
                        reach,
                    },
                    Some(Operation::Join {
                        left_on: first_left,
                        right_on: first_right,
                        output: first_output,
                        ..
                    }),
                ) = (&step.operation, first)
                else {
                    return false;
                };
                *left == before
                    && *right == before
                    && left_on == first_left
                    && right_on == first_right
                    && output == first_output
                    && filter.is_empty()
                    && view.is_empty()
                    && reach.is_none()
            });
            format!(
                "{} steps; {} links, each one join of the step before with itself, {}",
                plan.steps.len(),
                links.len(),
                if alike && links.len() == LADDER_LINKS {
                    "unfiltered"
                } else {
                    "not alike"
                }
            )
        }
        Ask::Names(plan) => {
            let plan = expanded(&plan());
            let last = plan
                .steps
                .last()
                .map(|step| named(&inputs_of(&step.operation)).to_vec());
            format!(
                "emit {}; frontier {:?}; observation {:?}; last reads {:?}",
                plan.emit.step.0,
                plan.frontier,
                plan.observations
                    .first()
                    .map(|observation| observation.source)
                    .expect("the row exports one output"),
                last.unwrap_or_default()
            )
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
