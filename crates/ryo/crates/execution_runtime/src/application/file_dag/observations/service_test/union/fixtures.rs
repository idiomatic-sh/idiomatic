//! The support of the `union` cases.
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::open_file;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    operating::errors::OperatingError,
    prepared_request::{service::assemble, types::ObservationIdentity},
    table_model::types::Rows,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
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
    row_texts::types::Cell,
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
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
        id: RuleId(format!("union-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("union-{index}").into()),
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

pub(super) fn plan(operation: Operation) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation,
        }],
        frontier: vec![],
        comparison_origins: vec![],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}

pub(super) fn exercise(given: &[&[i64]], expected: Option<&[i64]>, registered_calls: usize) {
    let consumer = given.len();
    let mut rules = (0..=consumer).map(rule).collect::<Vec<_>>();
    let mut plans = (0..consumer)
        .map(|index| {
            let mut plan = plan(Operation::Hosted {
                relation: HostedNumber(index as u32),
            });
            plan.observations.push(PlanObservation {
                name: ObservationName("values".into()),
                source: Input::Step(StepId(0)),
                columns: columns(),
                scope: StepScope::PerFile,
            });
            plan
        })
        .collect::<Vec<_>>();
    let producers = (0..consumer)
        .map(|index| ObservationProducer::Output {
            rule: rules[index].id.clone(),
            output: ObservationName("values".into()),
        })
        .collect();
    rules[consumer].observations.push(ObservationBinding {
        name: ObservationBindingName("values".into()),
        columns: columns(),
        producers,
    });
    let mut consumer_plan = plan(Operation::Select {
        from: Input::BoundObservation(ObservationBindingId(0)),
        comparisons: vec![],
        kept: vec![],
    });
    consumer_plan
        .observation_inputs
        .push(BoundObservationInput {
            name: ObservationBindingName("values".into()),
            columns: columns(),
            scope: if consumer == 0 {
                StepScope::CrossFile
            } else {
                StepScope::PerFile
            },
        });
    if consumer == 0 {
        // Assembly links compiled plans; the step itself retains its inferred phase.
        consumer_plan.steps[0].scope = StepScope::CrossFile;
    }
    plans.push(consumer_plan);
    let compiled =
        assemble(plans, vec![vec![]; rules.len()], &rules).expect("actual named request assembly");
    let hosted = given
        .iter()
        .enumerate()
        .map(|(index, rows)| HostRelation {
            name: HostName(format!("host-{index}").into()),
            columns: vec![HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Number,
            }],
            rows: rows
                .iter()
                .map(|number| HostRow(vec![Cell::Number(Integer(*number))]))
                .collect(),
        })
        .collect::<Vec<_>>();
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        compiled,
        &rules,
        &vec![Digest([0; 16]); rules.len()],
        &hosted,
        Arc::clone(&derived),
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([7; 16]))),
    )
    .expect("prepared named request");
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("named-union.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &(0..rules.len())
            .map(|index| FileRuleIntent::Run {
                diagnostic: if index == consumer {
                    DiagnosticDelivery::Collected
                } else {
                    DiagnosticDelivery::Disabled
                },
                observations: if index == consumer {
                    ObservationDemand::NotRequired
                } else {
                    ObservationDemand::Required
                },
                capture: CaptureIntent::Disabled,
            })
            .collect::<Vec<_>>(),
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    for index in 0..consumer {
        let outcome = file
            .rule(
                RuleIndex(index),
                RuleDemand {
                    emit: EmitDemand::Disabled,
                    observations: ObservationDemand::Required,
                },
                &read,
                &mut symbols,
            )
            .unwrap();
        assert!(
            matches!(outcome.observations[0].outcome, RootOutcome::Complete(_)),
            "the actual producer publishes its named root"
        );
    }
    let outcome = file
        .rule(
            RuleIndex(consumer),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            &mut symbols,
        )
        .unwrap();
    if let Some(expected) = expected {
        let RootOutcome::Complete(table) = outcome.emit else {
            panic!("all complete producers must supply the named union")
        };
        assert_eq!(
            table.cells,
            expected
                .iter()
                .map(|number| Value::Number(Integer(*number)))
                .collect::<Vec<_>>(),
            "{given:?}"
        );
        assert_eq!(table.count.0, expected.len());
        assert_eq!(
            file.observation_routes(ryo_values::domain::records::types::ObservationIndex(0))
                .unwrap()
                .registered_calls,
            registered_calls,
            "the top union dispatches once per input row, including singleton duplication"
        );
    } else {
        assert!(matches!(
            outcome.emit,
            RootOutcome::Deferred {
                phase: StepScope::CrossFile
            }
        ));
    }
    let registered = file
        .observation_routes(ryo_values::domain::records::types::ObservationIndex(0))
        .unwrap()
        .registered_calls;
    let readiness = file.readiness_counts();
    let repeated = file
        .rule(
            RuleIndex(consumer),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            &mut symbols,
        )
        .unwrap();
    match expected {
        Some(expected) => {
            let RootOutcome::Complete(table) = repeated.emit else {
                panic!("union replay must complete")
            };
            assert_eq!(
                table.cells,
                expected
                    .iter()
                    .map(|number| Value::Number(Integer(*number)))
                    .collect::<Vec<_>>()
            );
        }
        None => assert!(matches!(
            repeated.emit,
            RootOutcome::Deferred {
                phase: StepScope::CrossFile
            }
        )),
    }
    assert_eq!(
        file.observation_routes(ryo_values::domain::records::types::ObservationIndex(0))
            .unwrap()
            .registered_calls,
        registered
    );
    assert_eq!(file.readiness_counts(), readiness);
    file.finish().unwrap();
}
