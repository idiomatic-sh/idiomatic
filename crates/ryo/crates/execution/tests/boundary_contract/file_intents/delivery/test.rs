//! Observation capture and diagnostic delivery are bound before admission.
use crate::boundary_fixture::fixtures::Observed as _;
use crate::file_intents::fixtures::*;
use crate::*;
use ryo_execution::inbound::use_cases::execution::interface::Demand;
use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort;
use ryo_values::domain::runs::types::ApplicabilityRefusal;

#[test]
fn diagnostic_disabled_observation_producer_runs_and_captures_only_named_frontiers() {
    let engine = engine();
    let authored = named_request();
    let program = engine
        .prepare(authored)
        .expect("named producer and consumer");
    assert_eq!(
        program
            .demanded_rules(|rule| if rule.0 == 1 {
                Demand::Demanded
            } else {
                Demand::Undemanded
            })
            .0,
        [Demand::Demanded, Demand::Demanded]
    );
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let elements = ryo_language::domain::elements::types::Elements::default();
    let table = source();
    // Disabled diagnostic frontiers cannot read Identifier: the facts hold Function alone, and refuse any other read.
    let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Disabled,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::Observations,
        },
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        },
    ];
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut symbols,
        &mut facts,
    );
    let produced = ran(&mut rules, 0);
    assert!(
        matches!(produced.emit, EmittedRoot::Withheld),
        "no diagnostic"
    );
    let RuleAfter::Ran { captures } = produced.after else {
        panic!("the producer runs and captures")
    };
    assert_eq!(
        captures.len(),
        1,
        "only the declared named frontier is captured"
    );
    for capture in captures {
        capture.outcome.expect("named frontier publication");
    }
    assert!(matches!(
        root(ran(&mut rules, 1)),
        Ok(RootOutcome::Deferred {
            phase: StepScope::CrossFile
        }),
    ));
    let FileEnd::Finished(readiness) = rules.end else {
        panic!("the file finishes")
    };
    let readiness = readiness.observed("the finished file observed its readiness");
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut facts = TableFacts::new(&[]);
    for reason in [
        ApplicabilityRefusal::SyntaxUndetermined,
        ApplicabilityRefusal::MetadataUnavailable,
    ] {
        let intents = [FileRuleIntent::Refused(reason), FileRuleIntent::Undemanded];
        let mut symbols = Symbols::default();
        let rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &mut facts,
        );
        let FileEnd::Finished(readiness) = rules.end else {
            panic!("the file finishes")
        };
        let readiness = readiness.observed("named frontier readiness");
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
    assert_eq!(
        facts.reads(),
        0,
        "applicability refusal cannot read a provider"
    );
    request.finish();
}

#[test]
fn diagnostic_delivery_is_bound_before_admission_and_cannot_switch_after_publication() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let elements = ryo_language::domain::elements::types::Elements::default();
    let table = source();
    for diagnostic in [DiagnosticDelivery::Collected, DiagnosticDelivery::Forwarded] {
        let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::DiagnosticAndObservations,
        }];
        let mut symbols = Symbols::default();
        let mut rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &mut facts,
        );
        let expected = Table {
            kinds: vec![CellKind::Node],
            count: RowCount(1),
            cells: vec![table.cells[0]],
        };
        if diagnostic == DiagnosticDelivery::Forwarded {
            // The per-file route delivers a collected root; a forwarded one has no delivery there and is refused.
            assert!(matches!(
                root(ran(&mut rules, 0)),
                Err(FileStop::Operating(OperatingError::NotRun))
            ));
            continue;
        }
        let Ok(RootOutcome::Complete(actual)) = root(ran(&mut rules, 0)) else {
            panic!("the declared {diagnostic:?} mode delivers its root")
        };
        assert_eq!(actual, expected);
        assert!(facts.reads() > 0);
        assert!(matches!(rules.end, FileEnd::Finished(_)));
    }
    request.finish();
}
