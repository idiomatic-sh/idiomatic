//! The requests the gated and all-active runs are compared over, each of rules written as their steps over a tree whose kinds
//! are `file`, `word` and `other`; `absent-kind` names none of them.
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::Digest;

/// A kind no tree node of the file has.
const ABSENT: &str = "absent-kind";

/// A comparison of column `column` with the written text `written`.
fn compared(test: Test, column: u32, written: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(written.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// The node column of the tree's nodes kept by `comparisons`.
fn tree_nodes(comparisons: Vec<PlanComparison>) -> Operation {
    Operation::Select {
        from: Input::Relation(Relation::SyntaxTree),
        comparisons,
        kept: vec![ColumnIndex(0)],
    }
}

/// The node column of the tree's nodes of kind `kind`: column 1 holds the kind.
fn of_kind(kind: &str) -> Operation {
    tree_nodes(vec![compared(Test::Equal, 1, kind)])
}

/// The rows of step `left` with a row of step `right` of their node.
fn joined(left: u32, right: u32) -> Operation {
    Operation::Join {
        left: Input::Step(StepId(left)),
        right: Input::Step(StepId(right)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }
}

/// A rule of `steps`, each in each file, that emits its last step.
fn rule(steps: Vec<Operation>) -> Plan {
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        emit: Emit {
            step: StepId(u32::try_from(steps.len().saturating_sub(1)).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps: steps
            .into_iter()
            .map(|operation| PlanStep {
                scope: StepScope::PerFile,
                operation,
            })
            .collect(),
        comparison_origins: vec![],
        frontier: vec![],
    }
}

/// The request of `plans`, each the plan of a rule of its own.
fn requested(plans: Vec<Plan>) -> PrepareRequest {
    let count = plans.len();
    PrepareRequest {
        plans: PlanSource::Authored {
            plans,
            bindings: vec![vec![]; count],
        },
        rules: (0..count)
            .map(|index| RuleSpec {
                id: RuleId(format!("gated-{index}").into()),
                query: QueryBundle {
                    root: QueryDocumentSource {
                        id: QuerySourceId(format!("gated-{index}").into()),
                        text: "".into(),
                    },
                    modules: Default::default(),
                },
                applicability: RuleApplicability::Always,
                given: vec![],
                observations: vec![],
                spelled: RuleParameters("".into()),
                include: vec![],
                exclude: vec![],
            })
            .collect(),
        digests: (0..count)
            .map(|index| Digest([u8::try_from(index % 256).unwrap_or(0); 16]))
            .collect(),
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    }
}

/// The file's nodes with no node of the absent kind: an anti-join whose right input has no row.
pub(crate) fn antijoin_of_an_absent_kind() -> PrepareRequest {
    requested(vec![rule(vec![
        of_kind("file"),
        of_kind(ABSENT),
        Operation::Antijoin {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(1)),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
    ])])
}

/// The `other` nodes and the absent kind's: a union with one side of no row.
pub(crate) fn union_with_an_absent_kind() -> PrepareRequest {
    requested(vec![rule(vec![
        of_kind("other"),
        of_kind(ABSENT),
        Operation::Union {
            left: Input::Step(StepId(0)),
            right: Input::Step(StepId(1)),
        },
    ])])
}

/// Two rules sharing the select of the `word` nodes: one projects it, the other joins it with the absent kind's.
pub(crate) fn shared_stage_one_branch_absent() -> PrepareRequest {
    requested(vec![
        rule(vec![
            of_kind("word"),
            Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
        ]),
        rule(vec![of_kind("word"), of_kind(ABSENT), joined(0, 1)]),
    ])
}

/// The tree's nodes whose text matches the written pattern `[`, which the matcher refuses at its first row, joined with the
/// absent kind's.
pub(crate) fn refused_literal_pattern_joined_with_an_absent_kind() -> PrepareRequest {
    requested(vec![rule(vec![
        tree_nodes(vec![compared(Test::Matches, 2, "[")]),
        of_kind(ABSENT),
        joined(0, 1),
    ])])
}

/// The absent kind's nodes joined with the `word` nodes.
pub(crate) fn absent_kind_joined() -> PrepareRequest {
    requested(vec![rule(vec![
        of_kind(ABSENT),
        of_kind("word"),
        joined(0, 1),
    ])])
}

/// The absent kind's nodes alone.
pub(crate) fn absent_kind_alone() -> PrepareRequest {
    requested(vec![rule(vec![of_kind(ABSENT)])])
}

/// A RUF100-like observer: rules 0 and 1 each name their nodes of a kind, `word` and the absent kind, as their output `nodes`,
/// and rule 2 selects the rows of both outputs through one binding.
pub(crate) fn observer_of_two_producers_one_absent() -> PrepareRequest {
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("node".into()),
        kind: CellKind::Node,
    }];
    let producer = |kind: &str| {
        let mut plan = rule(vec![of_kind(kind)]);
        plan.frontier = vec![Input::Step(StepId(0))];
        plan.observations = vec![PlanObservation {
            name: ObservationName("nodes".into()),
            source: Input::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        }];
        plan
    };
    let mut observer = rule(vec![Operation::Select {
        from: Input::BoundObservation(ObservationBindingId(0)),
        comparisons: vec![],
        kept: vec![],
    }]);
    observer.observation_inputs = vec![BoundObservationInput {
        name: ObservationBindingName("nodes".into()),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    }];
    let mut request = requested(vec![producer("word"), producer(ABSENT), observer]);
    let producers = request.rules[..2]
        .iter()
        .map(|producer| ObservationProducer::Output {
            rule: producer.id.clone(),
            output: ObservationName("nodes".into()),
        })
        .collect();
    request.rules[2].observations = vec![ObservationBinding {
        name: ObservationBindingName("nodes".into()),
        columns,
        producers,
    }];
    request.observations = ObservationIdentity::Named(ObservationReadinessLayout(Digest([93; 16])));
    request
}
