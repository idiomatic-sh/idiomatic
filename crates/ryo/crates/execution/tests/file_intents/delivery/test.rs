//! Observation capture and diagnostic delivery are bound before admission.
use crate::boundary_fixture::Observed as _;
use ryo_execution::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityRefusal;
use super::super::*;

#[test]
fn diagnostic_disabled_observation_producer_runs_and_captures_only_named_frontiers() {
    let engine = engine();
    let authored = named_request();
    let program = engine
        .prepare(authored)
        .expect("named producer and consumer");
    assert_eq!(
        program.demanded_rules(|rule| if rule.0 == 1 {
            Demand::Demanded
        } else {
            Demand::Undemanded
        }),
        [Demand::Demanded, Demand::Demanded]
    );
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let table = source();
    let read = |relation| {
        assert_eq!(
            relation,
            Relation::Function,
            "disabled diagnostic frontiers cannot read Identifier"
        );
        Ok(SourceRows::Computed(&table))
    };
    let producer_roots = RuleDemand {
        emit: EmitDemand::Disabled,
        observations: ObservationDemand::Required,
    };
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Disabled,
            observations: producer_roots.observations,
            capture: CaptureIntent::Observations,
        },
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        },
    ];
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(input(&intents, &elements, &read), &mut symbols)
        .unwrap();
    assert!(matches!(
        file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!("no diagnostic")),
        Err(OperatingError::NotRun)
    ));
    let produced = file
        .rule(RuleIndex(0), producer_roots, &mut symbols)
        .unwrap();
    assert!(matches!(produced.emit, RootOutcome::NoQuery));
    assert!(matches!(
        produced.observations[0].outcome,
        RootOutcome::Complete(_)
    ));
    let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
    let captures = file
        .capture_frontiers(
            RuleIndex(0),
            ryo_values::domain::stored_facts::types::Bucket(0),
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words,
            },
        )
        .unwrap();
    assert_eq!(
        captures.len(),
        1,
        "only the declared named frontier is captured"
    );
    for capture in captures {
        capture.outcome.expect("named frontier publication");
        file.commit_frontier(RuleIndex(0), capture.input).unwrap();
    }
    assert!(matches!(
        file.rule(RuleIndex(1), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Deferred {
            phase: StepScope::CrossFile
        },
    ));
    let readiness = file.finish().unwrap().readiness.observed("the finished file observed its readiness");
    assert!(readiness.complete.0.iter().any(|bits| *bits != 0));
    assert!(readiness.skipped.0.is_empty());
    request.finish();
}

#[test]
fn file_start_refusal_preserves_unknown_readiness_without_a_skip_proof() {
    use ryo_values::domain::observations::fixtures::{frontier_state, validate_frontiers};
    use ryo_values::domain::records::types::{ObservationReadinessSlot, ProducerReadiness};
    let engine = engine();
    let program = engine.prepare(named_request()).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| panic!("applicability refusal cannot read a provider");
    for reason in [
        ApplicabilityRefusal::SyntaxUndetermined,
        ApplicabilityRefusal::MetadataUnavailable,
    ] {
        let intents = [FileRuleIntent::Refused(reason), FileRuleIntent::Undemanded];
        let mut symbols = Symbols::default();
        let file = request
            .open_file(input(&intents, &elements, &read), &mut symbols)
            .unwrap();
        let readiness = file
            .finish()
            .unwrap()
            .readiness
            .observed("named frontier readiness");
        let checked = validate_frontiers(&readiness, readiness.layout, readiness.slots).unwrap();
        assert!(matches!(
            frontier_state(checked, ObservationReadinessSlot(0)).unwrap(),
            ProducerReadiness::Unknown(_)
        ));
        assert!(
            readiness.skipped.0.is_empty(),
            "no positive applicability proof was issued"
        );
        assert!(readiness.complete.0.iter().all(|word| *word == 0));
    }
    request.finish();
}

#[test]
fn diagnostic_delivery_is_bound_before_admission_and_cannot_switch_after_publication() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let table = source();
    for diagnostic in [DiagnosticDelivery::Collected, DiagnosticDelivery::Forwarded] {
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            reads.set(reads.get() + 1);
            Ok(SourceRows::Computed(&table))
        };
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(input(&intents, &elements, &read), &mut symbols)
            .unwrap();
        match diagnostic {
            DiagnosticDelivery::Collected => assert!(matches!(
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!(
                    "wrong mode cannot deliver"
                )),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Forwarded => assert!(matches!(
                file.rule(RuleIndex(0), demand(), &mut symbols),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Disabled => unreachable!(),
        }
        assert_eq!(reads.get(), 0, "wrong mode cannot admit any provider");
        assert!(
            file.counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .all(|count| count.0 == [0; 3])
        );
        let expected = Table {
            kinds: vec![CellKind::Node],
            count: RowCount(1),
            cells: vec![table.cells[0]],
        };
        match diagnostic {
            DiagnosticDelivery::Collected => {
                let RootOutcome::Complete(actual) = file
                    .rule(RuleIndex(0), demand(), &mut symbols)
                    .unwrap()
                    .emit
                else {
                    panic!("correct declared collected mode remains runnable")
                };
                assert_eq!(actual, expected);
            }
            DiagnosticDelivery::Forwarded => {
                let mut rows = vec![];
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    rows.push(row.to_vec());
                    Ok(())
                })
                .expect("correct declared forwarded mode remains runnable");
                assert_eq!(rows, [expected.cells]);
            }
            DiagnosticDelivery::Disabled => unreachable!(),
        }
        let admitted_reads = reads.get();
        assert!(admitted_reads > 0);
        match diagnostic {
            DiagnosticDelivery::Collected => assert!(matches!(
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!(
                    "publication cannot change mode"
                )),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Forwarded => assert!(matches!(
                file.rule(RuleIndex(0), demand(), &mut symbols),
                Err(OperatingError::NotRun)
            )),
            DiagnosticDelivery::Disabled => unreachable!(),
        }
        assert_eq!(
            reads.get(),
            admitted_reads,
            "mode rejection cannot reread a published provider"
        );
        file.finish().unwrap();
    }
    request.finish();
}
