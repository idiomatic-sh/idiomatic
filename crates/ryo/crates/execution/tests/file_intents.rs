//! File-start declarations are enforced through every public execution route.
use super::*;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};

fn input<'a, 'facts>(
    intents: &'a [FileRuleIntent],
    elements: &'a ryo_language::domain::elements::types::Elements,
    read: &'a dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> FileInput<'a, 'facts> {
    FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: facts::ResidentStability::Stable,
        file: FileId(0),
        evaluation: &[],
        publication: &[],
        intents,
        source: b"abc",
        elements,
        read,
        semantic: None,
        kinds: None,
    }
}

#[test]
fn every_prepared_rule_requires_exactly_one_file_start_intent() {
    let engine = engine();
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| panic!("malformed intent vectors cannot read a provider");
    let mut symbols = Symbols::default();
    for intents in [
        &[][..],
        &[FileRuleIntent::Undemanded][..],
        &[FileRuleIntent::Undemanded; 3][..],
    ] {
        assert!(matches!(
            request.open_file(input(intents, &elements, &read), &mut symbols),
            Err(OperatingError::ShapeMismatch)
        ));
    }
    request.finish();
}

#[test]
fn absent_skipped_and_refused_rules_cannot_acquire_any_root() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| panic!("undeclared roots cannot read a provider");
    for intent in [
        FileRuleIntent::Undemanded,
        FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected),
        FileRuleIntent::Refused(ApplicabilityRefusal::SyntaxUndetermined),
        FileRuleIntent::Refused(ApplicabilityRefusal::MetadataUnavailable),
    ] {
        let intents = [intent];
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(input(&intents, &elements, &read), &mut symbols)
            .unwrap();
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols),
            Err(OperatingError::NotRun)
        ));
        assert!(matches!(
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!("no delivery")),
            Err(OperatingError::NotRun)
        ));
        let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
        assert!(matches!(
            file.capture_frontiers(
                RuleIndex(0),
                ryo_values::domain::stored_facts::types::Bucket(0),
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: &mut arena,
                    places: &mut places,
                    words: &mut words
                },
            ),
            Err(OperatingError::NotRun)
        ));
        assert!(matches!(
            file.commit_frontier(
                RuleIndex(0),
                ryo_values::domain::plan_parts::types::InputNumber(0)
            ),
            Err(OperatingError::NotRun)
        ));
        file.finish().unwrap();
    }
    request.finish();
}

#[test]
fn rule_demand_is_immutable_and_capture_cannot_be_added_after_open() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let reads = Cell::new(0);
    let table = source();
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        reads.set(reads.get() + 1);
        Ok(SourceRows::Computed(&table))
    };
    let intents = [FileRuleIntent::Run {
        roots: demand(),
        capture: CaptureIntent::Disabled,
    }];
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(input(&intents, &elements, &read), &mut symbols)
        .unwrap();
    for changed in [
        RuleDemand {
            emit: EmitDemand::Disabled,
            observations: ObservationDemand::NotRequired,
        },
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        },
    ] {
        assert!(matches!(
            file.rule(RuleIndex(0), changed, &mut symbols),
            Err(OperatingError::NotRun)
        ));
    }
    let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
    assert!(matches!(
        file.capture_frontiers(
            RuleIndex(0),
            ryo_values::domain::stored_facts::types::Bucket(0),
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: &mut arena,
                places: &mut places,
                words: &mut words
            },
        ),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(reads.get(), 0, "refused root changes do not admit a source");
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Complete(_)
    ));
    assert!(reads.get() > 0);
    file.finish().unwrap();
    request.finish();
}

fn named_request() -> PreparedRequest {
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingId, ObservationBindingName,
        ObservationColumn, ObservationColumnName, ObservationName, ObservationProducer,
        ObservationReadinessLayout, PlanObservation,
    };
    let mut authored = authored_request(2);
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("node".into()),
        kind: CellKind::Node,
    }];
    let producer = &mut authored.plans[0];
    producer.steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: Input::Relation(Relation::Identifier),
            comparisons: vec![],
            kept: vec![ColumnIndex(0)],
        },
    });
    producer.steps.push(PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::Step(StepId(1)),
            comparisons: vec![],
            kept: vec![],
        },
    });
    producer.emit.step = StepId(2);
    producer.frontier = vec![Input::Step(StepId(0)), Input::Step(StepId(1))];
    producer.observations.push(PlanObservation {
        name: ObservationName("nodes".into()),
        source: Input::Step(StepId(0)),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    });
    authored.plans[1].steps[0].scope = StepScope::CrossFile;
    authored.plans[1].steps[0].operation = Operation::Select {
        from: Input::BoundObservation(ObservationBindingId(0)),
        comparisons: vec![],
        kept: vec![],
    };
    authored.plans[1]
        .observation_inputs
        .push(BoundObservationInput {
            name: ObservationBindingName("input".into()),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        });
    let producer_id = authored.rules[0].id.clone();
    authored.rules[1].observations.push(ObservationBinding {
        name: ObservationBindingName("input".into()),
        columns,
        producers: vec![ObservationProducer::Output {
            rule: producer_id,
            output: ObservationName("nodes".into()),
        }],
    });
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([91; 16])));
    authored
}

#[test]
fn diagnostic_disabled_observation_producer_runs_and_captures_only_named_frontiers() {
    let engine = engine();
    let authored = named_request();
    let program = engine
        .prepare(authored)
        .expect("named producer and consumer");
    assert_eq!(program.demanded_rules(|place| place == 1), [true, true]);
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
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
            roots: producer_roots,
            capture: CaptureIntent::Observations,
        },
        FileRuleIntent::Run {
            roots: demand(),
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
        produced.observations[0].1,
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
                arena: &mut arena,
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
    let readiness = file.finish().unwrap().readiness.unwrap();
    assert!(readiness.complete.iter().any(|bits| *bits != 0));
    assert!(readiness.skipped.is_empty());
    request.finish();
}

#[test]
fn file_start_refusal_preserves_unknown_readiness_without_a_skip_proof() {
    use ryo_values::domain::observations::{
        service::{frontier_state, validate_frontiers},
        types::{ObservationReadinessSlot, ProducerReadiness},
    };
    let engine = engine();
    let program = engine.prepare(named_request()).unwrap();
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
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
            .expect("named frontier readiness");
        let checked = validate_frontiers(&readiness, readiness.layout, readiness.slots).unwrap();
        assert!(matches!(
            frontier_state(checked, ObservationReadinessSlot(0)).unwrap(),
            ProducerReadiness::Unknown(_)
        ));
        assert!(
            readiness.skipped.is_empty(),
            "no positive applicability proof was issued"
        );
        assert!(readiness.complete.iter().all(|word| *word == 0));
    }
    request.finish();
}
