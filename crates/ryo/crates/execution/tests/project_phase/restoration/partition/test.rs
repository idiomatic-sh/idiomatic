//! Actual two-file named restoration and absence publication, including a
//! downstream projection that would erase the completeness key if fused early.
use super::super::super::*;
use ryo_execution::inbound::use_cases::execution::interface::{
    ObservationExecution, ObservationPublication,
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::{
    files::types::CorpusInput,
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingId, ObservationBindingName,
        ObservationColumn, ObservationColumnName, ObservationIndex, ObservationName,
        ObservationProducer, ObservationReadinessLayout, PlanObservation, ProducerReadiness,
    },
    plan_parts::types::{HostedNumber, RepeatedRows},
    records::types::{Cell as HostCell, CrossPublicationTransfer, PackedObservedRoot},
    runs::types::{RuleApplicability, SkipReason, Skipped},
    stored_facts::types::{ArenaBytes, Digest, KeptWords},
    table_words::{
        service::{encode_table, pack},
        types::WordSlice,
    },
};

#[test]
fn named_absence_keeps_good_file_bags_and_orders_while_bad_file_stays_unknown() {
    for empty in [false, true] {
        for project_away_file in [false, true] {
            exercise(empty, project_away_file);
        }
    }
}

fn exercise(empty: bool, project_away_file: bool) {
    let engine = engine();
    let mut authored = authored_request(3);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let file_column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("file".into()),
        kind: CellKind::File,
    }];
    authored.plans[0].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(file_column as u32)],
    };
    authored.plans[0].frontier = vec![Input::Step(StepId(0))];
    authored.plans[0].observations = vec![PlanObservation {
        name: ObservationName("files".into()),
        source: Input::Step(StepId(0)),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    }];
    // A real empty project producer makes the declared union project-scoped.
    // It is evaluated and installed through the same public phase protocol.
    authored.plans[1].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
    }];
    authored.plans[1].observations = vec![PlanObservation {
        name: ObservationName("files".into()),
        source: Input::Step(StepId(0)),
        columns: columns.clone(),
        scope: StepScope::CrossFile,
    }];
    authored.plans[2].observation_inputs = vec![BoundObservationInput {
        name: ObservationBindingName("files".into()),
        columns: columns.clone(),
        scope: StepScope::CrossFile,
    }];
    authored.rules[2].observations = vec![ObservationBinding {
        name: ObservationBindingName("files".into()),
        columns,
        producers: (0..2)
            .map(|rule| ObservationProducer::Output {
                rule: authored.rules[rule].id.clone(),
                output: ObservationName("files".into()),
            })
            .collect(),
    }];
    authored.plans[2].steps = vec![
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Hosted {
                relation: HostedNumber(1),
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::BoundObservation(ObservationBindingId(0)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
        },
    ];
    if project_away_file {
        authored.plans[2].steps.push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Project {
                from: Input::Step(StepId(1)),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            },
        });
    }
    authored.plans[2].emit.step = StepId(if project_away_file { 2 } else { 1 });
    let host_column = |name: &str, kind| HostColumn {
        name: HostColumnName(name.into()),
        kind,
    };
    authored.hosted = vec![
        HostRelation {
            name: HostName("empty-project".into()),
            columns: vec![host_column("file", HostKind::File)],
            rows: vec![],
        },
        HostRelation {
            name: HostName("candidates".into()),
            columns: vec![
                host_column("file", HostKind::File),
                host_column("value", HostKind::Number),
            ],
            rows: [
                ("second.py", 10),
                ("input.py", 20),
                ("input.py", 20),
                ("second.py", 40),
                ("input.py", 30),
            ]
            .into_iter()
            .map(|(file, value)| {
                HostRow(vec![
                    HostCell::File(FilePath(file.into())),
                    HostCell::Number(Integer(value)),
                ])
            })
            .collect(),
        },
    ];
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([97; 16])));
    let producer_id = authored.rules[0].id.clone();
    let prepared = engine.prepare(authored).expect("real mixed named program");
    let mut corpus = corpus();
    let mut second = corpus[0].clone();
    let CorpusInput::Available(entry) = &mut second else {
        unreachable!()
    };
    entry.path = FilePath("second.py".into());
    corpus.push(second);
    let request = engine.begin(&prepared, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut table = source();
    if empty {
        table.cells.clear();
        table.count = RowCount(0);
    }
    let read = |_| Ok(SourceRows::Computed(&table));
    let mut captures = vec![];
    let mut readiness = vec![];
    for file_id in [FileId(0), FileId(1)] {
        let mut symbols = Symbols::default();
        let intents = [
            if file_id == FileId(0) {
                FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Disabled,
                    observations: ObservationDemand::Required,
                    capture: CaptureIntent::Observations,
                }
            } else {
                FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)
            },
            FileRuleIntent::Undemanded,
            FileRuleIntent::Undemanded,
        ];
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: file_id,
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .unwrap();
        if file_id == FileId(0) {
            let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
            let capture = file
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
                .unwrap()
                .pop()
                .unwrap();
            file.commit_frontier(RuleIndex(0), capture.input).unwrap();
            captures.push((file_id, capture.outcome.unwrap()));
        }
        readiness.push(FileObservationReadiness {
            file: file_id,
            root: file.finish().unwrap().readiness.unwrap(),
        });
    }
    let skipped = vec![Skipped {
        rule: producer_id,
        path: FilePath("second.py".into()),
        reason: SkipReason::SyntaxRejected,
    }];
    let stored = CapturedFacts {
        captures,
        reads: AtomicUsize::new(0),
    };
    let project_facts = ProjectFacts::new(Relation::ModuleIdentity);
    let project_symbols = Symbols::default();
    let partials = Partials::default();
    let mut arenas = vec![];
    let mut stage = request.observations(&mut arenas).unwrap();
    stage.finish(RuleIndex(0)).unwrap();
    stage
        .restore_frontiers(
            &stored,
            &corpus,
            &[],
            &skipped,
            &readiness,
            &[Bucket(0), Bucket(10), Bucket(20)],
            &[false, true, true],
        )
        .unwrap();
    let mut symbols = Symbols::default();
    let scope = stage
        .project_scope(RuleIndex(1), &corpus, &[], &skipped)
        .unwrap();
    let slot = stage.reserve(RuleIndex(1)).unwrap();
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(1),
                first: Bucket(10),
                facts: &stored,
                project: &project_facts,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    let mut project = request
        .open_project(
            ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: stage.arenas(),
            },
            &mut symbols,
        )
        .unwrap();
    let mut outcome = project
        .rule(
            RuleIndex(1),
            RuleDemand {
                emit: EmitDemand::Disabled,
                observations: ObservationDemand::Required,
            },
            &mut symbols,
        )
        .unwrap();
    let (output, ProjectObservation::Complete { table, publication }) =
        outcome.observations.pop().unwrap()
    else {
        panic!("the empty project member is a real complete publication")
    };
    assert_eq!(table.count, RowCount(0));
    project.finish();
    let PublicationSlot::Reserved(worker) = slot else {
        panic!("demanded publication arena")
    };
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    stage
        .install(
            RuleIndex(1),
            slot,
            CrossPublicationTransfer {
                worker,
                arena: ArenaBytes::default(),
                roots: vec![PackedObservedRoot {
                    output: ObservationIndex(0),
                    words: Some(KeptWords(pack(WordSlice(&words.0)))),
                    readiness: ProducerReadiness::Complete,
                    arenas: vec![],
                }],
            },
            vec![(output, publication)],
        )
        .unwrap();
    stage.finish(RuleIndex(1)).unwrap();
    drop(scope);
    let scope = stage
        .project_scope(RuleIndex(2), &corpus, &[], &skipped)
        .unwrap();
    let bound = stage.bindings(RuleIndex(2)).unwrap();
    assert_eq!(
        stage
            .binding_readiness(RuleIndex(2))
            .unwrap()
            .get(&ObservationBindingId(0)),
        Some(&true)
    );
    let ordinary = request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule: RuleIndex(2),
                first: Bucket(20),
                facts: &stored,
                project: &project_facts,
                project_symbols: &project_symbols,
            },
            &mut symbols,
        )
        .unwrap();
    let mut project = request
        .open_project(
            ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &bound,
                arenas: stage.arenas(),
            },
            &mut symbols,
        )
        .unwrap();
    for _retry in 0..2 {
        let RootOutcome::Partial { table, affected } = project
            .rule(RuleIndex(2), demand(), &mut symbols)
            .unwrap()
            .emit
        else {
            panic!("only the bad physical partition must remain explicitly unknown")
        };
        assert_eq!(affected, vec![Some(FileId(1))]);
        let expected = if !empty {
            vec![]
        } else if project_away_file {
            vec![
                Value::Number(Integer(20)),
                Value::Number(Integer(20)),
                Value::Number(Integer(30)),
            ]
        } else {
            vec![
                Value::File(FileId(0)),
                Value::Number(Integer(20)),
                Value::File(FileId(0)),
                Value::Number(Integer(20)),
                Value::File(FileId(0)),
                Value::Number(Integer(30)),
            ]
        };
        assert_eq!(
            table.cells, expected,
            "good-file multiplicity and order survive missing named rows"
        );
        assert_eq!(table.count, RowCount(if empty { 3 } else { 0 }));
    }
    assert_eq!(project_facts.reads.load(Ordering::SeqCst), 0);
    project.finish();
    drop(bound);
    drop(scope);
    stage.finish(RuleIndex(2)).unwrap();
    stage.finish_stage().unwrap();
    request.finish();
}
