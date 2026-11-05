//! Warm replay closure is observed through the actual prepared facade, not a copied graph algorithm.
use crate::application::file_dag::program::facade;
use crate::domain::{
    operating::errors::OperatingError,
    prepared_request::{service::assemble, types::ObservationIdentity},
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    observations::types::{
        BoundObservationSchema, ObservationBinding, ObservationBindingId, ObservationBindingName,
        ObservationColumn, ObservationColumnName, ObservationName, ObservationProducer,
        ObservationReadinessLayout, PlanObservation,
    },
    plan_parts::types::{ColumnIndex, HostedNumber, Input, StepId},
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    records::types::{Cell, PlacedRule},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::{Bucket, Digest},
    stores::types::{MadeAgain, Reuse},
};
use std::{collections::BTreeMap, sync::Arc};
fn columns() -> Vec<ObservationColumn> {
    vec![ObservationColumn {
        name: ObservationColumnName("value".into()),
        kind: CellKind::Number,
    }]
}
fn rule(index: usize) -> RuleSpec {
    RuleSpec {
        id: RuleId(format!("replay-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("replay-{index}").into()),
                text: "".into(),
            },
            modules: BTreeMap::new(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}
fn fixture(per_file: bool) -> Arc<facade::PreparedProgram> {
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
            inputs.push(BoundObservationSchema {
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
    facade::prepare(
        compiled,
        &rules,
        &[Digest([0; 16]); 4],
        &hosted,
        crate::application::file_dag::testing::program_derived_port(),
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([9; 16]))),
    )
    .unwrap()
}
fn selection(indices: &[u32]) -> Reuse {
    Reuse::Cross(MadeAgain::These(
        indices.iter().copied().map(PlacedRule).collect(),
    ))
}
#[test]
fn warm_consumer_selects_transitive_cross_producers_and_changed_producer_selects_consumers() {
    let prepared = fixture(false);
    let first = [Bucket(10), Bucket(20), Bucket(30), Bucket(40)];
    assert_eq!(
        prepared.replay_closure(selection(&[2]), &first).unwrap(),
        (selection(&[0, 1, 2]), vec![])
    );
    assert_eq!(
        prepared.replay_closure(selection(&[0]), &first).unwrap(),
        (selection(&[0, 1, 2]), vec![])
    );
    assert_eq!(
        prepared.replay_closure(selection(&[3]), &first).unwrap(),
        (selection(&[3]), vec![])
    );
    assert_eq!(
        prepared.replay_closure(selection(&[]), &first).unwrap(),
        (selection(&[]), vec![])
    );
    assert!(matches!(
        prepared.replay_closure(selection(&[4]), &first),
        Err(OperatingError::NotRun)
    ));
}
#[test]
fn per_file_producer_is_not_reexecuted_in_the_consumer_cross_phase() {
    let prepared = fixture(true);
    assert_eq!(
        prepared
            .replay_closure(
                selection(&[2]),
                &[Bucket(10), Bucket(20), Bucket(30), Bucket(40)]
            )
            .unwrap(),
        (selection(&[1, 2]), vec![Bucket(10)]),
        "actual captured file frontier is read instead of rerunning its producer"
    );
}
