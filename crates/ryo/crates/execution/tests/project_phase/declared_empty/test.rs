//! Closed empty declarations remain typed populations; absent declarations remain refusals.
use super::super::*;
use ryo_execution::inbound::use_cases::execution::interface::ObservationExecution;
use ryo_values::domain::{
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingId, ObservationBindingName,
        ObservationColumn, ObservationColumnName, ObservationName, ObservationProducer,
    },
    runs::types::RuleId,
    stored_facts::types::Digest,
};

fn declared() -> PreparedRequest {
    let mut input = authored_request(1);
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("code".into()),
        kind: CellKind::Symbol,
    }];
    input.rules[0].observations.push(ObservationBinding {
        name: ObservationBindingName("codes".into()),
        columns: columns.clone(),
        producers: vec![],
    });
    input.plans[0]
        .observation_inputs
        .push(BoundObservationInput {
            name: ObservationBindingName("codes".into()),
            columns,
            scope: StepScope::CrossFile,
        });
    input.plans[0].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Select {
            from: Input::BoundObservation(ObservationBindingId(0)),
            comparisons: vec![],
            kept: vec![],
        },
    }];
    input.plans[0].frontier = vec![Input::BoundObservation(ObservationBindingId(0))];
    input.observations = ObservationIdentity::Named(
        ryo_values::domain::observations::types::ObservationReadinessLayout(Digest([9; 16])),
    );
    input
}

#[test]
fn declared_empty_observation_defers_in_file_and_publishes_typed_empty_in_project() {
    let engine = engine();
    let prepared = engine
        .prepare(declared())
        .expect("closed empty declaration");
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let reads = Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Err::<SourceRows<'_>, _>(OperatingError::NotRead)
    };
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Deferred {
            phase: StepScope::CrossFile
        }
    ));
    let mut arena = vec![];
    let mut places = vec![];
    let mut words = Words::default();
    let captures = file
        .capture_frontiers(
            RuleIndex(0),
            Bucket(0),
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: &mut arena,
                places: &mut places,
                words: &mut words,
            },
        )
        .unwrap();
    assert!(
        captures.is_empty(),
        "project population is not fabricated as a file frame"
    );
    assert_eq!(reads.get(), 0);
    file.finish().unwrap();
    assert_eq!(reads.get(), 0);

    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let mut arenas = vec![];
    let mut stage = request.observations(&mut arenas).unwrap();
    assert_eq!(
        stage
            .binding_readiness(RuleIndex(0))
            .unwrap()
            .get(&ObservationBindingId(0)),
        Some(&true)
    );
    let bound = stage.bindings(RuleIndex(0)).unwrap();
    let scope = stage
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &ryo_execution::inbound::use_cases::execution::types::InapplicableFiles::default(),
        )
        .unwrap();
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    // Two declared roots of the one rule share the invocation; each prepares its own frontiers.
    let inputs = (0..2)
        .map(|_| ProjectInput {
            scope: &scope,
            evaluation: &partials,
            publication: &partials,
            ordinary: request
                .prepare_frontiers(
                    FrontierRequest {
                        demand: FrontierDemand::Diagnostic,
                        scope: &scope,
                        rule: RuleIndex(0),
                        first: Bucket(0),
                        facts: &stored,
                        project: &project,
                        project_symbols: &project_symbols,
                    },
                    &mut symbols,
                )
                .unwrap(),
            bound: &bound,
            arenas: stage.arenas(),
        })
        .collect();
    for outcome in crate::boundary_fixture::project_roots(&stage, RuleIndex(0), demand(), inputs, &mut symbols)
    {
        let RootOutcome::Complete(table) = outcome.unwrap().emit else {
            panic!("authoritatively empty union must complete in its declared phase")
        };
        assert_eq!(table.kinds, [CellKind::Symbol]);
        assert_eq!(table.count, RowCount(0));
        assert!(table.cells.is_empty());
    }
    assert_eq!(stored.reads.load(Ordering::SeqCst), 0);
    assert_eq!(project.reads.load(Ordering::SeqCst), 0);
    stage.finish(RuleIndex(0)).unwrap();
}

#[test]
fn no_query_is_unavailable_rather_than_an_empty_population() {
    let mut input = declared();
    input.rules[0].observations[0].producers = vec![ObservationProducer::NoQuery {
        rule: RuleId("not-executable".into()),
    }];
    project_refuses(input, "observation producer has no executable query", false);
}

fn project_refuses(input: PreparedRequest, expected: &str, ready: bool) {
    let engine = engine();
    let prepared = engine
        .prepare(input)
        .expect("valid declaration with deferred admission refusal");
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let stored = CapturedFacts {
        captures: vec![],
        reads: AtomicUsize::new(0),
    };
    let project = ProjectFacts::new(Relation::ModuleIdentity);
    let covered = [Covered {
        file: FileId(0),
        rules: vec![RuleIndex(0)],
        partial: vec![],
    }];
    let mut arenas = vec![];
    let stage = request.observations(&mut arenas).unwrap();
    assert_eq!(
        stage
            .binding_readiness(RuleIndex(0))
            .unwrap()
            .get(&ObservationBindingId(0)),
        Some(&ready)
    );
    let bound = stage.bindings(RuleIndex(0)).unwrap();
    let scope = stage
        .project_scope(
            RuleIndex(0),
            &corpus,
            &covered,
            &[],
            &ryo_execution::inbound::use_cases::execution::types::InapplicableFiles::default(),
        )
        .unwrap();
    let mut symbols = Symbols::default();
    let project_symbols = Symbols::default();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(0),
                first: Bucket(0),
                facts: &stored,
                project: &project,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    let partials = Partials::default();
    let outcome = crate::boundary_fixture::project_root(
        &stage,
        RuleIndex(0),
        demand(),
        ProjectInput {
            scope: &scope,
            evaluation: &partials,
            publication: &partials,
            ordinary,
            bound: &bound,
            arenas: stage.arenas(),
        },
        &mut symbols,
    );
    assert!(
        matches!(outcome.unwrap().emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason })
        if reason == expected)
    );
    assert_eq!(stored.reads.load(Ordering::SeqCst), 0);
    assert_eq!(project.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn missing_declarations_outputs_and_mismatched_schemas_are_not_closed_empty() {
    let engine = engine();
    let mut missing = declared();
    missing.rules[0].observations.clear();
    assert!(matches!(
        engine.prepare(missing),
        Err(OperatingError::ShapeMismatch)
    ));
    let mut missing_slot = declared();
    missing_slot.plans[0].steps[0].operation = Operation::Select {
        from: Input::BoundObservation(ObservationBindingId(1)),
        comparisons: vec![],
        kept: vec![],
    };
    assert!(matches!(
        engine.prepare(missing_slot),
        Err(OperatingError::ObservationConfiguration(
            FileProgramError::InvalidReference
        ))
    ));
    let mut mismatch = declared();
    mismatch.plans[0].observation_inputs[0].columns[0].kind = CellKind::Number;
    project_refuses(
        mismatch,
        "observation does not match its actual consumer declaration",
        true,
    );

    let mut unknown = declared();
    unknown.rules[0].observations[0].producers = vec![ObservationProducer::Output {
        rule: RuleId("missing".into()),
        output: ObservationName("codes".into()),
    }];
    assert!(matches!(
        engine.prepare(unknown),
        Err(OperatingError::ObservationConfiguration(
            FileProgramError::UnknownProducer { .. }
        ))
    ));
    let mut missing_output = declared();
    let other = authored_request(2);
    missing_output.rules.push(other.rules[1].clone());
    missing_output.plans.push(other.plans[1].clone());
    missing_output.digests.push(other.digests[1]);
    missing_output.bindings.push(vec![]);
    missing_output.rules[0].observations[0].producers = vec![ObservationProducer::Output {
        rule: missing_output.rules[1].id.clone(),
        output: ObservationName("codes".into()),
    }];
    assert!(matches!(
        engine.prepare(missing_output),
        Err(OperatingError::ObservationConfiguration(
            FileProgramError::MissingOutput { .. }
        ))
    ));
}
