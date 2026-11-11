//! The support of the `physical` cases, which the interpreter's twin cases also read.
use crate::boundary_fixture::fixtures::authored_request;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{ColumnIndex, HostedNumber, Input, JoinColumn, Side, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    row_texts::types::Cell,
};

pub(crate) fn ordered() -> PrepareRequest {
    let mut authored = authored_request(2);
    authored.hosted = [vec![1, 5], vec![2, 6]]
        .into_iter()
        .enumerate()
        .map(|(index, values)| HostRelation {
            name: HostName(format!("ordered-{index}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("order".into()),
                kind: HostKind::Number,
            }],
            rows: values
                .into_iter()
                .map(|value| HostRow(vec![Cell::Number(Integer(value))]))
                .collect(),
        })
        .collect();
    for plan in crate::boundary_fixture::fixtures::plans(&mut authored) {
        plan.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(0),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(1),
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::OrderedMatch {
                    left: Input::Step(StepId(0)),
                    right: Input::Step(StepId(1)),
                    left_on: vec![],
                    right_on: vec![],
                    left_order: ColumnIndex(0),
                    right_order: ColumnIndex(0),
                    output: vec![
                        JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(0),
                        },
                        JoinColumn {
                            side: Side::Right,
                            column: ColumnIndex(0),
                        },
                    ],
                },
            },
        ];
        plan.emit.step = StepId(2);
    }
    authored
}
