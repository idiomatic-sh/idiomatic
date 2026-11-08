//! Real request compilation and file opening must not allocate dormant siblings.
use super::super::*;
use ryo_values::domain::{
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
    },
    plan_parts::types::{ObservationBindingId, Operand, QueryLiteral},
    records::types::ObservationReadinessLayout,
};

#[test]
fn opening_and_preparing_one_rule_uses_only_its_reached_metadata_and_preserves_aliases() {
    const PRODUCERS: usize = 128;
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("node".into()),
        kind: CellKind::Node,
    }];
    let mut rules = Vec::new();
    let mut plans = Vec::new();
    for literal in 0..PRODUCERS {
        let mut authored = rule();
        authored.id = RuleId(format!("opening-{literal}").into());
        rules.push(authored);
        let mut producer = plan(Scenario::ProjectRetry);
        producer.steps = vec![
            producer.steps[0].clone(),
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Extend {
                    from: PlanInput::Step(StepId(0)),
                    values: vec![Operand::Literal(QueryLiteral::Number(Integer(
                        literal as i64,
                    )))],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(1)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
        ];
        producer.emit.step = StepId(2);
        producer.observations = vec![PlanObservation {
            name: ObservationName("nodes".into()),
            source: PlanInput::Step(StepId(2)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        }];
        plans.push(producer);
    }
    let mut consumer = rule();
    consumer.id = RuleId("opening-consumer".into());
    consumer.observations = vec![ObservationBinding {
        name: ObservationBindingName("input".into()),
        columns: columns.clone(),
        producers: rules
            .iter()
            .map(|rule| ObservationProducer::Output {
                rule: rule.id.clone(),
                output: ObservationName("nodes".into()),
            })
            .collect(),
    }];
    rules.push(consumer);
    let mut consumer_plan = plans[0].clone();
    consumer_plan.steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: PlanInput::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    consumer_plan.observations.clear();
    consumer_plan.observation_inputs = vec![BoundObservationInput {
        name: ObservationBindingName("input".into()),
        columns,
        scope: StepScope::PerFile,
    }];
    consumer_plan.emit.step = StepId(0);
    plans.push(consumer_plan);
    let compiled = crate::domain::prepared_request::service::assemble(
        plans,
        vec![vec![]; rules.len()],
        &rules,
    )
    .unwrap();
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        compiled,
        &rules,
        &vec![Digest([0; 16]); rules.len()],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([33; 16]))),
    )
    .unwrap();
    let roots = prepared.file_demand(RuleIndex(0)).unwrap();
    use crate::domain::executable_program::file_demand::types::{
        FileDemand, FrontierSelection, RootSelection,
    };
    assert_eq!(roots.selected(FileDemand::NONE).readers.len(), 0);
    assert_eq!(
        roots
            .selected(FileDemand {
                diagnostic: RootSelection::Required,
                observations: RootSelection::Required,
                capture: FrontierSelection::All
            })
            .readers
            .len(),
        3
    );
    assert_eq!(
        roots
            .selected(FileDemand {
                diagnostic: RootSelection::Required,
                observations: RootSelection::Required,
                capture: FrontierSelection::All
            })
            .roots
            .len(),
        3,
        "reader aliases remain distinct in the compiled obligation multiset"
    );
    assert_eq!(
        roots
            .selected(FileDemand {
                capture: FrontierSelection::Observations,
                ..FileDemand::NONE
            })
            .roots
            .len(),
        1,
        "observation-only capture retains its declared frontier"
    );
    assert!(
        roots
            .selected(FileDemand {
                capture: FrontierSelection::Observations,
                ..FileDemand::NONE
            })
            .observations
            .is_empty()
    );
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("opening.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut intents = vec![FileRuleIntent::Undemanded; rules.len()];
    intents[0] = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::Required,
        capture: CaptureIntent::DiagnosticAndObservations,
    };
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &intents,
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        file.roots.len(),
        1,
        "unrequested named outputs have no file-owned state"
    );
    assert_eq!(file.prepared_named.len(), 1);
    assert_eq!(
        file.prepared_roots.len(),
        1,
        "Emit/named/frontier aliases queue their shared root once"
    );
    assert_eq!(
        file.execution.state_counts().slots.0,
        0,
        "opening admits no executable region"
    );
    assert_eq!(
        file.execution.state_counts().dormant_obligations.0,
        0,
        "disabled siblings do not populate cancellation counters at file opening"
    );
    let rows = function_rows(&[2]);
    let reads = std::cell::Cell::new(0);
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        reads.set(reads.get() + 1);
        Ok(Rows::Computed(&rows))
    };
    assert!(matches!(
        file.rule(
            RuleIndex(1),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::Required
            },
            &read,
            &mut symbols
        ),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        reads.get(),
        0,
        "a disabled sibling cannot acquire its source"
    );
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
    let RootOutcome::Complete(actual) = outcome.emit else {
        panic!("declared diagnostic root")
    };
    assert_eq!(
        actual.cells,
        vec![Cell::Node(node_id(FileId(0), Position(2)))]
    );
    assert!(matches!(
        outcome.observations[0].outcome,
        RootOutcome::Complete(_)
    ));
    assert!(
        file.execution.state_counts().slots.0 <= 3,
        "unrequested program siblings remain untouched"
    );
    assert!(file.readiness_counts().evaluations.0 <= 3);
    file.finish().unwrap();
}
