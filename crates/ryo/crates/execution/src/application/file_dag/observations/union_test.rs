//! Named producer populations use the actual prepared union and common file scheduler.
use crate::application::file_dag::{
    execution::CommonOperatorEngine,
    program::{
        facade::{self, EmitDemand, ObservationDemand, RootOutcome, RuleDemand},
        facade_runtime::{ProgramServices, open_file},
    },
    scope::ScopePartials,
};
use crate::domain::{
    coverage::types::RuleIndex,
    operating::errors::OperatingError,
    prepared_request::{service::assemble, types::ObservationIdentity},
    table_model::types::Rows,
};
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingId, ObservationBindingName,
        ObservationColumn, ObservationColumnName, ObservationName, ObservationProducer,
        ObservationReadinessLayout, PlanObservation,
    },
    plan_parts::types::{ColumnIndex, HostedNumber, Input, StepId},
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    records::types::Cell,
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
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
        id: RuleId(format!("union-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("union-{index}").into()),
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
fn plan(operation: Operation) -> Plan {
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
#[test]
fn singleton_and_many_producers_preserve_distinct_union_order_through_one_prepared_root() {
    let cases: &[(&[&[i64]], &[i64])] = &[
        (&[&[]], &[]),
        (&[&[1, 1, 2]], &[1, 2]),
        (&[&[1, 1, 2], &[]], &[1, 2]),
        (&[&[], &[1, 1, 2]], &[1, 2]),
        (&[&[1, 2], &[2, 3], &[1, 3]], &[1, 2, 3]),
    ];
    for &(given, expected) in cases {
        exercise(given, Some(expected));
    }
}
#[test]
fn zero_producers_refuse_instead_of_fabricating_an_admitted_empty_population() {
    exercise(&[], None);
}
fn exercise(given: &[&[i64]], expected: Option<&[i64]>) {
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
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
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
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
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
        &vec![true; rules.len()],
        services,
        b"",
        true,
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
            matches!(outcome.observations[0].1, RootOutcome::Complete(_)),
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
            file.observation_routes(0).unwrap().registered_calls,
            1,
            "the prepared union root is evaluated once"
        );
    } else {
        assert!(matches!(
            outcome.emit,
            RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
        ));
    }
    file.finish().unwrap();
}
