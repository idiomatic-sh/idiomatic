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

#[derive(Clone, Copy, Debug)]
enum Mode {
    PartialRight,
    PartialLeft { difference: bool },
    Rekey { file_key: bool },
    Current,
}

#[test]
fn named_absence_keeps_good_file_bags_and_orders_while_bad_file_stays_unknown() {
    for empty in [false, true] {
        for project_away_file in [false, true] {
            for stale_complete in [false, true] {
                exercise(empty, project_away_file, stale_complete, Mode::PartialRight);
            }
        }
    }
}
#[test]
fn partial_left_and_complete_empty_right_preserve_known_positive_rows() {
    for project_away_file in [false, true] {
        for difference in [false, true] {
            exercise(
                false,
                project_away_file,
                false,
                Mode::PartialLeft { difference },
            );
        }
    }
}
#[test]
fn empty_rekeying_join_widens_uncertainty_but_file_equality_keeps_good_files_known() {
    for file_key in [false, true] {
        exercise(true, false, false, Mode::Rekey { file_key });
    }
}
#[test]
fn current_partial_project_publication_retains_owner_provenance_through_real_consumer() {
    for project_away_file in [false, true] {
        exercise(true, project_away_file, false, Mode::Current);
    }
}

fn exercise(empty: bool, project_away_file: bool, stale_complete: bool, mode: Mode) {
    let engine = engine();
    let mut authored = authored_request(3);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let file_column = kinds(Relation::Function)
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
            kind: CellKind::Number,
        },
    ];
    authored.plans[0].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(file_column as u32)],
    };
    authored.plans[0].steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Extend {
            from: Input::Step(StepId(0)),
            values: vec![ryo_values::domain::plan_parts::types::Operand::Literal(
                ryo_values::domain::plan_parts::types::QueryLiteral::Number(Integer(7)),
            )],
        },
    });
    authored.plans[0].frontier = vec![Input::Step(StepId(1))];
    authored.plans[0].observations = vec![PlanObservation {
        name: ObservationName("files".into()),
        source: Input::Step(StepId(1)),
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
    if matches!(mode, Mode::Current) {
        let column = kinds(Relation::ModuleIdentity)
            .iter()
            .position(|kind| *kind == CellKind::File)
            .unwrap();
        authored.plans[1].steps[0].operation = Operation::Select {
            from: Input::Relation(Relation::ModuleIdentity),
            comparisons: vec![],
            kept: vec![ColumnIndex(column as u32)],
        };
        authored.plans[1].steps.push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Extend {
                from: Input::Step(StepId(0)),
                values: vec![ryo_values::domain::plan_parts::types::Operand::Literal(
                    ryo_values::domain::plan_parts::types::QueryLiteral::Number(Integer(7)),
                )],
            },
        });
    }
    authored.plans[1].observations = vec![PlanObservation {
        name: ObservationName("files".into()),
        source: Input::Step(StepId(if matches!(mode, Mode::Current) { 1 } else { 0 })),
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
    if let Mode::PartialLeft { difference } = mode {
        authored.plans[2].steps[0].operation = Operation::Hosted {
            relation: HostedNumber(0),
        };
        let left = Input::BoundObservation(ObservationBindingId(0));
        let right = Input::Step(StepId(0));
        authored.plans[2].steps[1].operation = if difference {
            Operation::Difference { left, right }
        } else {
            Operation::Antijoin {
                left,
                right,
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            }
        };
    }
    if let Mode::Rekey { file_key } = mode {
        let key = ColumnIndex(if file_key { 0 } else { 1 });
        authored.plans[2].steps[1] = PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Hosted {
                relation: HostedNumber(2),
            },
        };
        authored.plans[2].steps.push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Join {
                left: Input::BoundObservation(ObservationBindingId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![key],
                right_on: vec![key],
                output: vec![
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(0),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        });
        authored.plans[2].steps.push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Antijoin {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(2)),
                left_on: vec![ColumnIndex(0), ColumnIndex(1)],
                right_on: vec![ColumnIndex(0), ColumnIndex(1)],
            },
        });
    }
    if matches!(mode, Mode::Current) {
        if let Operation::Antijoin {
            left_on, right_on, ..
        } = &mut authored.plans[2].steps[1].operation
        {
            *left_on = vec![ColumnIndex(0), ColumnIndex(1)];
            *right_on = left_on.clone();
        }
    }
    if project_away_file {
        let previous = authored.plans[2].steps.len() - 1;
        authored.plans[2].steps.push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Project {
                from: Input::Step(StepId(previous as u32)),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            },
        });
    }
    authored.plans[2].emit.step = StepId((authored.plans[2].steps.len() - 1) as u32);
    let host_column = |name: &str, kind| HostColumn {
        name: HostColumnName(name.into()),
        kind,
    };
    authored.hosted = vec![
        HostRelation {
            name: HostName("empty-project".into()),
            columns: vec![
                host_column("file", HostKind::File),
                host_column("code", HostKind::Number),
            ],
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
    if matches!(mode, Mode::Rekey { .. }) {
        let row = HostRow(vec![
            HostCell::File(FilePath("input.py".into())),
            HostCell::Number(Integer(7)),
        ]);
        authored.hosted[1].rows = vec![row.clone()];
        authored.hosted.push(HostRelation {
            name: HostName("rekey-targets".into()),
            columns: authored.hosted[1].columns.clone(),
            rows: vec![row],
        });
    }
    if matches!(mode, Mode::Current) {
        authored.hosted[1].rows = [
            ("second.py", 7),
            ("input.py", 7),
            ("input.py", 8),
            ("second.py", 8),
            ("input.py", 8),
        ]
        .into_iter()
        .map(|(file, code)| {
            HostRow(vec![
                HostCell::File(FilePath(file.into())),
                HostCell::Number(Integer(code)),
            ])
        })
        .collect();
    }
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
    if stale_complete {
        // Model persisted Complete bits for a now rejected file. Only current
        // physical provenance can place the uncertainty; stale bits cannot erase it.
        readiness[1].root = Arc::clone(&readiness[0].root);
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
    let publication_partials = if matches!(mode, Mode::Current) {
        use ryo_execution::inbound::use_cases::metadata::interface::{FilePartial, Lacking};
        Partials {
            entries: vec![FilePartial {
                file: Some(FileId(1)),
                partial: Partial {
                    relation: Relation::ModuleIdentity,
                    cause: ryo_values::domain::stored_facts::types::PartialCause(
                        "missing project partition".into(),
                    ),
                    lacking: Lacking::Any,
                    at: Some(FileId(1)),
                },
            }],
            unavailable_files: vec![],
        }
    } else {
        Partials::default()
    };
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
                publication: &publication_partials,
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
    let (output, observed) = outcome.observations.pop().unwrap();
    let (table, publication, readiness) = match (mode, observed) {
        (Mode::Current, ProjectObservation::Partial { table, publication }) => {
            assert_eq!(
                table.cells,
                vec![Value::File(FileId(0)), Value::Number(Integer(7))]
            );
            (
                table,
                publication,
                ProducerReadiness::Unknown(
                    ryo_values::domain::observations::types::ObservationReason(
                        "physical source facts are incomplete".into(),
                    ),
                ),
            )
        }
        (_, ProjectObservation::Complete { table, publication }) => {
            assert_eq!(table.count, RowCount(0));
            (table, publication, ProducerReadiness::Complete)
        }
        _ => panic!("the actual project population keeps its exact completeness"),
    };
    project.finish();
    let PublicationSlot::Reserved(worker) = slot else {
        panic!("demanded publication arena")
    };
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    let transfer = CrossPublicationTransfer {
        worker,
        arena: ArenaBytes::default(),
        roots: vec![PackedObservedRoot {
            output,
            words: Some(KeptWords(pack(WordSlice(&words.0)))),
            readiness,
            arenas: vec![],
        }],
    };
    if matches!(mode, Mode::Current) {
        let mut forged = transfer.clone();
        forged.roots[0].readiness = ProducerReadiness::Complete;
        assert!(
            stage
                .install(
                    RuleIndex(1),
                    slot,
                    forged,
                    vec![(output, publication.clone())]
                )
                .is_err(),
            "a Partial owner proof cannot be relabeled Complete"
        );
    }
    stage
        .install(RuleIndex(1), slot, transfer, vec![(output, publication)])
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
        assert_eq!(
            affected,
            if matches!(mode, Mode::Rekey { file_key: false }) {
                vec![None]
            } else {
                vec![Some(FileId(1))]
            }
        );
        let values = match mode {
            Mode::PartialLeft { .. } => vec![7],
            Mode::Rekey { file_key: true } => vec![7],
            Mode::Rekey { file_key: false } => vec![],
            Mode::Current => vec![8, 8],
            Mode::PartialRight if empty => vec![20, 20, 30],
            Mode::PartialRight => vec![],
        };
        let expected = values
            .iter()
            .flat_map(|value| {
                if project_away_file {
                    vec![Value::Number(Integer(*value))]
                } else {
                    vec![Value::File(FileId(0)), Value::Number(Integer(*value))]
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            table.cells, expected,
            "known rows preserve their multiplicity and order for {mode:?}"
        );
        assert_eq!(table.count, RowCount(values.len()));
    }
    assert_eq!(
        project_facts.reads.load(Ordering::SeqCst),
        usize::from(matches!(mode, Mode::Current))
    );
    project.finish();
    drop(bound);
    drop(scope);
    stage.finish(RuleIndex(2)).unwrap();
    stage.finish_stage().unwrap();
    request.finish();
}
