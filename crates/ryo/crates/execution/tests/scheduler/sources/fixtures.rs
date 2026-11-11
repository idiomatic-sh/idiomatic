//! The support of the `sources` cases, which the interpreter's twin cases also read.
use crate::boundary_fixture::fixtures::authored_request;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest as _;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_relations::domain::relations::types::*;
use ryo_values::domain::{
    observations::types::*,
    plan_parts::types::{Input, ObservationBindingId, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    records::types::ObservationReadinessLayout,
    stored_facts::types::Digest,
};

pub(crate) fn named() -> PrepareRequest {
    let mut authored = authored_request(3);
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("node".into()),
        kind: CellKind::Node,
    }];
    crate::boundary_fixture::fixtures::plans(&mut authored)[0].observations =
        vec![PlanObservation {
            name: ObservationName("nodes".into()),
            source: Input::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        }];
    crate::boundary_fixture::fixtures::plans(&mut authored)[1].observation_inputs =
        vec![BoundObservationInput {
            name: ObservationBindingName("nodes".into()),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        }];
    crate::boundary_fixture::fixtures::plans(&mut authored)[1].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Union {
            left: Input::BoundObservation(ObservationBindingId(0)),
            right: Input::BoundObservation(ObservationBindingId(0)),
        },
    }];
    // The linker retains exports only when another rule reads them, and its
    // canonical export contract uses Step roots. Keep the per-file union as a
    // real export; a separately demanded cross-file step reads the bound input
    // directly, making that Observation source an actual file frontier.
    crate::boundary_fixture::fixtures::plans(&mut authored)[1]
        .steps
        .push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Union {
                left: Input::BoundObservation(ObservationBindingId(0)),
                right: Input::BoundObservation(ObservationBindingId(0)),
            },
        });
    crate::boundary_fixture::fixtures::plans(&mut authored)[1].observations = vec![
        PlanObservation {
            name: ObservationName("forwarded".into()),
            source: Input::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        },
        PlanObservation {
            name: ObservationName("projected".into()),
            source: Input::Step(StepId(1)),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut authored)[2].observation_inputs = vec![
        BoundObservationInput {
            name: ObservationBindingName("forwarded".into()),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        },
        BoundObservationInput {
            name: ObservationBindingName("projected".into()),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        },
    ];
    crate::boundary_fixture::fixtures::plans(&mut authored)[2].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Union {
            left: Input::BoundObservation(ObservationBindingId(0)),
            right: Input::BoundObservation(ObservationBindingId(1)),
        },
    }];
    authored.rules[2].observations = ["forwarded", "projected"]
        .into_iter()
        .map(|name| ObservationBinding {
            name: ObservationBindingName(name.into()),
            columns: columns.clone(),
            producers: vec![ObservationProducer::Output {
                rule: authored.rules[1].id.clone(),
                output: ObservationName(name.into()),
            }],
        })
        .collect();
    authored.rules[1].observations = vec![ObservationBinding {
        name: ObservationBindingName("nodes".into()),
        columns,
        producers: vec![ObservationProducer::Output {
            rule: authored.rules[0].id.clone(),
            output: ObservationName("nodes".into()),
        }],
    }];
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([117; 16])));
    authored
}
