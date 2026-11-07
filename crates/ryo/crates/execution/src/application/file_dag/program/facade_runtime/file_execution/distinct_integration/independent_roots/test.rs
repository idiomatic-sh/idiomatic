//! Real semantic refusal at one compiled root cannot erase the independent root.
use super::super::*;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::records::types::ProducerReadiness;
#[derive(Debug)]
struct RefuseCount;
impl OperatorsUseCase for RefuseCount {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        if matches!(
            &call,
            OperatorCall::ReduceCount { .. }
                | OperatorCall::CheckAggregation { .. }
                | OperatorCall::StreamAggregation { .. }
                | OperatorCall::FinishAggregation { .. }
        ) {
            return Err(PortableOperatorFault::WrongOperation);
        }
        crate::outbound::operators::fixtures::fake::OperatorsFake::default().operators(call)
    }
}
fn columns(failed_emit: bool) -> Vec<ObservationColumn> {
    let mut columns = vec![ObservationColumn {
        name: ObservationColumnName("subject".into()),
        kind: CellKind::Node,
    }];
    if !failed_emit {
        columns.push(ObservationColumn {
            name: ObservationColumnName("count".into()),
            kind: CellKind::Number,
        });
    }
    columns
}
#[test]
fn failed_emit_and_failed_observation_do_not_erase_the_independent_root() {
    for failed_emit in [true, false] {
        let output_columns = columns(failed_emit);
        let mut producer = plan(Scenario::ProjectRetry);
        producer.steps = vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(0)],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::GroupCount {
                    from: PlanInput::Step(StepId(0)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Dropped,
                },
            },
        ];
        producer.emit.step = StepId(if failed_emit { 1 } else { 2 });
        producer.observations.push(PlanObservation {
            name: ObservationName("independent".into()),
            source: PlanInput::Step(StepId(if failed_emit { 2 } else { 1 })),
            columns: output_columns.clone(),
            scope: StepScope::PerFile,
        });
        let mut consumer = plan(Scenario::ProjectRetry);
        consumer.steps.truncate(1);
        consumer.emit.step = StepId(0);
        consumer.steps[0].operation = Operation::Select {
            from: PlanInput::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        };
        consumer.observation_inputs.push(BoundObservationInput {
            name: ObservationBindingName("input".into()),
            columns: output_columns.clone(),
            scope: StepScope::PerFile,
        });
        let producer_rule = rule();
        let mut consumer_rule = rule();
        consumer_rule.id = RuleId("independent-consumer".into());
        consumer_rule.observations.push(ObservationBinding {
            name: ObservationBindingName("input".into()),
            columns: output_columns,
            producers: vec![ObservationProducer::Output {
                rule: producer_rule.id.clone(),
                output: ObservationName("independent".into()),
            }],
        });
        let rules = [producer_rule, consumer_rule];
        let compiled = crate::domain::prepared_request::service::assemble(
            vec![producer, consumer],
            vec![vec![], vec![]],
            &rules,
        )
        .unwrap();
        let derived = crate::application::file_dag::testing::program_derived_port();
        let prepared = crate::application::file_dag::program::preparation::service::prepare(
            compiled,
            &rules,
            &[Digest([0; 16]); 2],
            &[],
            Arc::clone(&derived),
            ObservationIdentity::Named(ObservationReadinessLayout(Digest([6; 16]))), crate::domain::executable_program::types::RoutePolicy::default(),
        )
        .unwrap();
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(Arc::new(RefuseCount))),
            derived,
            &[CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: FilePath("independent-roots.py".into()),
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
            &[
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::Required,
                    capture: CaptureIntent::Disabled,
                },
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::Required,
                    capture: CaptureIntent::Disabled,
                },
            ],
            services,
            ryo_values::domain::files::types::SourceView(b""),
            crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
            &mut symbols,
            None,
            None)
        .unwrap();
        let table = function_rows(&[0]);
        let read = |relation| {
            if relation == Relation::Function {
                Ok(Rows::Computed(&table))
            } else {
                Err(OperatingError::NotRead)
            }
        };
        let outcome = file
            .rule(
                RuleIndex(0),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::Required,
                },
                &read,
                &mut symbols,
            )
            .unwrap();
        assert_eq!(outcome.observations.len(), 1);
        assert_eq!(outcome.observations[0].output, ObservationIndex(0));
        if failed_emit {
            assert!(matches!(
                outcome.emit,
                RootOutcome::Failed(OperatingError::WrongOperation)
            ));
            let RootOutcome::Complete(observed) = &outcome.observations[0].outcome else {
                panic!("independent named root must publish despite emit refusal")
            };
            assert_eq!(observed.count.0, 1);
            assert_eq!(
                observed.cells,
                [Cell::Node(node_id(FileId(0), Position(0)))]
            );
            assert_eq!(
                file.roots[&RuleIndex(0)][0].readiness,
                Some(ProducerReadiness::Complete)
            );
        } else {
            let RootOutcome::Complete(emitted) = outcome.emit else {
                panic!("independent diagnostic root must publish despite named-root refusal")
            };
            assert_eq!(emitted.count.0, 1);
            assert_eq!(emitted.cells, [Cell::Node(node_id(FileId(0), Position(0)))]);
            assert!(matches!(
                &outcome.observations[0].outcome,
                RootOutcome::Failed(OperatingError::WrongOperation)
            ));
            assert!(matches!(
                &file.roots[&RuleIndex(0)][0].readiness,
                Some(ProducerReadiness::Unknown(_))
            ));
            assert!(file.roots[&RuleIndex(0)][0].publication.is_none());
        }
        file.finish().unwrap();
    }
}
