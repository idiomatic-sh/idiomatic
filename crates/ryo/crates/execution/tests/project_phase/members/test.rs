//! S11: a binding fed by two producers routes each member to its own producer, in the project and in a file.
use crate::boundary_fixture::fixtures::{authored_request, corpus, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::{
    BatchNumber, BatchOccurrence, ObservationBatches,
};
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::CapturedFacts;
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_source_facts::ProjectFacts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use std::sync::atomic::AtomicUsize;

/// S11: a binding fed by two producers routes each member to its own producer's publication: the consumer reads both producers'
/// rows, each once.
#[test]
fn each_member_of_a_binding_reads_its_own_producer() {
    use ryo_values::domain::plan_parts::types::{LiteralText, Operand, QueryLiteral};
    let engine = engine();
    let mut authored = authored_request(3);
    let relation = Relation::ModuleIdentity;
    let file_column = kinds(relation)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        },
        ObservationColumn {
            name: ObservationColumnName("code".into()),
            kind: CellKind::Symbol,
        },
    ];
    for (producer, code) in [(0, "A"), (1, "B")] {
        let plan = &mut crate::boundary_fixture::fixtures::plans(&mut authored)[producer];
        plan.steps = vec![
            PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Select {
                    from: Input::Relation(relation),
                    comparisons: vec![],
                    kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
                },
            },
            PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Extend {
                    from: Input::Step(StepId(0)),
                    values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                        code.into(),
                    )))],
                },
            },
        ];
        plan.emit.step = StepId(1);
        plan.observations.push(PlanObservation {
            name: ObservationName("files".into()),
            source: Input::Step(StepId(1)),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        });
    }
    let producers = [0, 1]
        .map(|rule| ObservationProducer::Output {
            rule: authored.rules[rule].id.clone(),
            output: ObservationName("files".into()),
        })
        .to_vec();
    let consumer = &mut crate::boundary_fixture::fixtures::plans(&mut authored)[2];
    consumer.steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    consumer.frontier = vec![Input::BoundObservation(ObservationBindingId(0))];
    consumer.observation_inputs.push(BoundObservationInput {
        name: ObservationBindingName("files".into()),
        columns: columns.clone(),
        scope: StepScope::CrossFile,
    });
    authored.rules[2].observations.push(ObservationBinding {
        name: ObservationBindingName("files".into()),
        columns,
        producers,
    });
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([64; 16])));
    let prepared = engine
        .prepare(authored)
        .expect("two producers and one consumer");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0), RuleIndex(1), RuleIndex(2)],
        partial: vec![],
    }];
    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(relation);
    let project_symbols = Symbols::default();
    let inapplicable = vec![
            ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default();
            3
        ];
    let refusals =
        vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default(); 3];
    let mut arenas = vec![];
    let stage = request.observations(&mut arenas).unwrap();
    let mut symbols = Symbols::default();
    let occurrence = |batch, rule| BatchOccurrence {
        batch: BatchNumber(batch),
        rule: RuleIndex(rule),
    };
    let (ran, mut delivered) = crate::project_phase::source_facts::fixtures::run_observations(
        stage,
        ObservationBatches {
            occurrences: &[occurrence(0, 0), occurrence(0, 1), occurrence(1, 2)],
            readiness: &[],
            rules: &[RuleIndex(0), RuleIndex(1), RuleIndex(2)],
            first: &[Bucket(0), Bucket(1), Bucket(2)],
            corpus: &corpus,
            covered: &covered,
            skipped: &[],
            inapplicable: &inapplicable,
            refused: &refusals,
            facts: &stored,
            project: &project,
            project_symbols: &project_symbols,
        },
        &Partials::default(),
        &mut symbols,
    );
    ran.expect("both producers publish and the consumer reads them");
    let (_, consumed) = delivered.pop().unwrap();
    let RootOutcome::Complete(table) = consumed.expect("the consumer is bound").emit else {
        panic!("the consumer completes")
    };
    assert_eq!(table.count, RowCount(2), "one row from each producer");
    let codes = table
        .cells
        .chunks_exact(2)
        .map(|row| row[1])
        .collect::<Vec<_>>();
    assert!(
        matches!(codes.as_slice(), [Value::Name(a), Value::Name(b)] if a != b),
        "the two rows carry their own producer's code: {codes:?}"
    );
    request.finish();
}

/// S11, in a file: a per-file binding fed by two producers of the same file routes each member to its own producer's rows.
#[test]
fn each_member_of_a_file_binding_reads_its_own_producer() {
    use ryo_values::domain::plan_parts::types::{LiteralText, Operand, QueryLiteral};
    let engine = engine();
    let mut authored = authored_request(3);
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName("node".into()),
            kind: CellKind::Node,
        },
        ObservationColumn {
            name: ObservationColumnName("code".into()),
            kind: CellKind::Symbol,
        },
    ];
    for (producer, code) in [(0, "A"), (1, "B")] {
        let plan = &mut crate::boundary_fixture::fixtures::plans(&mut authored)[producer];
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Extend {
                from: Input::Step(StepId(0)),
                values: vec![Operand::Literal(QueryLiteral::Text(LiteralText(
                    code.into(),
                )))],
            },
        });
        plan.emit.step = StepId(1);
        plan.observations.push(PlanObservation {
            name: ObservationName("nodes".into()),
            source: Input::Step(StepId(1)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        });
    }
    let producers = [0, 1]
        .map(|rule| ObservationProducer::Output {
            rule: authored.rules[rule].id.clone(),
            output: ObservationName("nodes".into()),
        })
        .to_vec();
    let consumer = &mut crate::boundary_fixture::fixtures::plans(&mut authored)[2];
    consumer.steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    consumer.observation_inputs.push(BoundObservationInput {
        name: ObservationBindingName("nodes".into()),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    });
    authored.rules[2].observations.push(ObservationBinding {
        name: ObservationBindingName("nodes".into()),
        columns,
        producers,
    });
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([65; 16])));
    let prepared = engine
        .prepare(authored)
        .expect("two file producers and one consumer");
    let produce = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::Required,
        capture: CaptureIntent::Observations,
    };
    let consume = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    };
    let intents = [produce, produce, consume];
    let (consumed, _) = crate::file_intents::fixtures::answered_rule(
        &prepared,
        &[(Relation::Function, source())],
        (&[], &[]),
        &intents,
        RuleIndex(2),
    );
    let Ok(RootOutcome::Complete(table)) = consumed else {
        panic!("the consumer of two file producers completes")
    };
    assert_eq!(table.count, RowCount(2), "one row from each producer");
    let codes = table
        .cells
        .chunks_exact(2)
        .map(|row| row[1])
        .collect::<Vec<_>>();
    assert!(
        matches!(codes.as_slice(), [Value::Symbol(a), Value::Symbol(b)] if a != b),
        "the two rows carry their own producer's code: {codes:?}"
    );
}
