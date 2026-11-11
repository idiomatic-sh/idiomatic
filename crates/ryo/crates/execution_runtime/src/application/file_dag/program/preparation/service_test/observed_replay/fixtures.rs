//! The support of the `observed_replay` cases.
use ryo_execution_domain::domain::prepared_request::{
    service::assemble, types::ObservationIdentity,
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
    },
    plan_parts::types::{
        ColumnIndex, HostedNumber, Input, ObservationBindingId, StepId, StepScope,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    records::types::ObservationReadinessLayout,
    row_texts::types::{Cell, PlacedRule},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
    stores::types::{MadeAgain, Reuse},
};
use std::sync::Arc;

pub(super) fn columns() -> Vec<ObservationColumn> {
    vec![ObservationColumn {
        name: ObservationColumnName("value".into()),
        kind: CellKind::Number,
    }]
}

pub(super) fn rule(index: usize) -> RuleSpec {
    RuleSpec {
        id: RuleId(format!("replay-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("replay-{index}").into()),
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
    }
}

pub(super) fn fixture(
    per_file: bool,
) -> Arc<crate::application::file_dag::program::preparation::types::PreparedProgram> {
    let mut rules = (0..4).map(rule).collect::<Vec<_>>();
    let mut plans = Vec::new();
    for index in 0..4 {
        let phase = if index == 0 && per_file {
            StepScope::PerFile
        } else {
            StepScope::CrossFile
        };
        let mut inputs = vec![];
        let operation = if index == 1 || index == 2 {
            let producer = index - 1;
            let producer_phase = if producer == 0 && per_file {
                StepScope::PerFile
            } else {
                StepScope::CrossFile
            };
            let producer_id = rules[producer].id.clone();
            rules[index].observations.push(ObservationBinding {
                name: ObservationBindingName("input".into()),
                columns: columns(),
                producers: vec![ObservationProducer::Output {
                    rule: producer_id,
                    output: ObservationName("output".into()),
                }],
            });
            inputs.push(BoundObservationInput {
                name: ObservationBindingName("input".into()),
                columns: columns(),
                scope: producer_phase,
            });
            Operation::Select {
                from: Input::BoundObservation(ObservationBindingId(0)),
                comparisons: vec![],
                kept: vec![],
            }
        } else {
            Operation::Hosted {
                relation: HostedNumber(0),
            }
        };
        plans.push(Plan {
            observation_inputs: inputs,
            observations: if index < 2 {
                vec![PlanObservation {
                    name: ObservationName("output".into()),
                    source: Input::Step(StepId(0)),
                    columns: columns(),
                    scope: phase,
                }]
            } else {
                vec![]
            },
            parameters: vec![],
            steps: vec![PlanStep {
                scope: phase,
                operation,
            }],
            emit: Emit {
                step: StepId(0),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
            comparison_origins: vec![],
        });
    }
    let compiled =
        assemble(plans, vec![vec![]; 4], &rules).expect("real named dependency assembly");
    let hosted = [HostRelation {
        name: HostName("numbers".into()),
        columns: vec![HostColumn {
            name: HostColumnName("value".into()),
            kind: HostKind::Number,
        }],
        rows: vec![HostRow(vec![Cell::Number(
            ryo_relations::domain::relations::types::Integer(1),
        )])],
    }];
    crate::application::file_dag::program::preparation::service::prepare(
        compiled,
        &rules,
        &[Digest([0; 16]); 4],
        &hosted,
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([9; 16]))),
    )
    .unwrap()
}

pub(super) fn selection(indices: &[u32]) -> Reuse {
    Reuse::Cross(MadeAgain::These(
        indices.iter().copied().map(PlacedRule).collect(),
    ))
}
