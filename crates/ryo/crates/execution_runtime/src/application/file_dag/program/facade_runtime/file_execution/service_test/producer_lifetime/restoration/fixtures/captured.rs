//! The captured project a restoration scenario restores from.
use super::*;
pub(in super::super) struct CapturedProject {
    pub(in super::super) prepared: Arc<PreparedProgram>,
    pub(in super::super) phase: PublicationScope,
    services: Arc<ProgramServices>,
    required: RequiredPartitions,
    bound: Vec<BoundInput>,
    refused: Vec<BoundInput>,
}

impl CapturedProject {
    pub(in super::super) fn new(exit: Exit, symbols: &mut Symbols) -> Result<Self, OperatingError> {
        let columns = [CellKind::File, CellKind::Number, CellKind::Number]
            .into_iter()
            .zip(["file", "left", "right"])
            .map(|(kind, name)| ObservationColumn {
                name: ObservationColumnName(name.into()),
                kind,
            })
            .collect::<Vec<_>>();
        let mut producer = shared_plan(ProjectionOrder::Forward, FinalConsumer::Completes);
        producer.steps.truncate(2);
        producer.emit.step = StepId(1);
        producer.frontier = vec![PlanInput::Step(StepId(1))];
        producer.observations = vec![PlanObservation {
            name: ObservationName("captured".into()),
            source: PlanInput::Step(StepId(1)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        }];
        let mut rules = vec![rule(); 6];
        for (at, rule) in rules.iter_mut().enumerate() {
            rule.id = RuleId(format!("restored-lifecycle-{at}").into());
        }
        let producer_id = rules[0].id.clone();
        let project_producer_id = rules[5].id.clone();
        let mut plans = vec![producer.clone()];
        for at in 1..5 {
            let mut plan = if at < 3 {
                let mut plan = producer.clone();
                plan.observations.clear();
                plan
            } else {
                shared_plan(
                    if at == 3 {
                        ProjectionOrder::Forward
                    } else {
                        ProjectionOrder::Reversed
                    },
                    if at == 4 && matches!(exit, Exit::SemanticFailure) {
                        FinalConsumer::InvalidTerminal
                    } else {
                        FinalConsumer::Completes
                    },
                )
            };
            for step in &mut plan.steps {
                step.scope = StepScope::CrossFile;
            }
            let Operation::Project { from, .. } = &mut plan.steps[0].operation else {
                panic!("actual shared producer starts with Project")
            };
            *from = PlanInput::BoundObservation(ObservationBindingId(0));
            plan.frontier = vec![PlanInput::BoundObservation(ObservationBindingId(0))];
            plan.observation_inputs = vec![BoundObservationInput {
                name: ObservationBindingName("source".into()),
                columns: columns.clone(),
                scope: StepScope::CrossFile,
            }];
            rules[at].observations = vec![ObservationBinding {
                name: ObservationBindingName("source".into()),
                columns: columns.clone(),
                producers: vec![
                    ObservationProducer::Output {
                        rule: producer_id.clone(),
                        output: ObservationName("captured".into()),
                    },
                    ObservationProducer::Output {
                        rule: project_producer_id.clone(),
                        output: ObservationName("project".into()),
                    },
                ],
            }];
            if at >= 3 {
                // The actual named Union is also an external phase obligation.
                // Its Join reader must not release required backing at delivery.
                plan.observations = vec![PlanObservation {
                    name: ObservationName("retained".into()),
                    source: PlanInput::Step(StepId(1)),
                    columns: columns.clone(),
                    scope: StepScope::CrossFile,
                }];
            }
            plans.push(plan);
        }
        // A per-file-only binding remains a file frontier: it cannot be
        // evaluated in a project population. Include a real empty project
        // producer so this mixed named union has the CrossFile contract used
        // by the actual restored-member ProjectRun route.
        let mut project_producer = producer.clone();
        project_producer.steps = vec![PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
        }];
        project_producer.emit.step = StepId(0);
        project_producer.frontier.clear();
        project_producer.observations = vec![PlanObservation {
            name: ObservationName("project".into()),
            source: PlanInput::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        }];
        plans.push(project_producer);
        let hosted = [HostRelation {
            name: HostName("empty-project-member".into()),
            columns: [HostKind::File, HostKind::Number, HostKind::Number]
                .into_iter()
                .zip(["file", "left", "right"])
                .map(|(kind, name)| HostColumn {
                    name: HostColumnName(name.into()),
                    kind,
                })
                .collect(),
            rows: vec![].into(),
        }];
        let compiled = ryo_execution_domain::domain::prepared_request::service::assemble(
            plans,
            vec![vec![]; rules.len()],
            &rules,
        )?;
        let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
        let prepared = crate::application::file_dag::program::preparation::service::prepare(
            compiled,
            &rules,
            &vec![Digest([0; 16]); rules.len()],
            &hosted,
            Arc::clone(&derived),
            ObservationIdentity::Named(ObservationReadinessLayout(Digest([91; 16]))),
        )?;
        let corpus = [CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("restored-lifecycle.input".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })];
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(Arc::new(
                crate::application::file_dag::fixtures::operators(),
            ))),
            derived,
            &corpus,
        ));
        let mut intents = [FileRuleIntent::Undemanded; 6];
        intents[0] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Disabled,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::Observations,
        };
        let source = Table {
            kinds: kinds(Relation::LanguageTarget).to_vec(),
            cells: vec![
                Cell::File(FileId(0)),
                Cell::Number(Integer(2)),
                Cell::Number(Integer(9)),
                Cell::File(FileId(0)),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(7)),
            ],
            count: RowCount(2),
        };
        let read = |relation| {
            if relation == Relation::LanguageTarget {
                Ok(Rows::Computed(&source))
            } else {
                Err(OperatingError::NotRead)
            }
        };
        let mut file = open_file(
            &prepared,
            FileId(0),
            ScopePartials::File {
                evaluation: &[],
                publication: &[],
            },
            &intents,
            Arc::clone(&services),
            ryo_values::domain::files::types::SourceView(b""),
            ReuseEligibility::Eligible,
            symbols,
            None,
            None,
        )?;
        let mut arena = vec![];
        let mut places = vec![];
        let mut words = ryo_relations::domain::relations::types::Words::default();
        let mut captures = file.capture_frontiers(
            RuleIndex(0),
            Bucket(0),
            &read,
            symbols,
            ryo_execution_domain::domain::frontier_capture::types::CaptureScope {
                file: FileId(0),
                rule_digest: Digest([0; 16]),
                worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                retained: ryo_execution_domain::domain::finish::types::Retained {
                    arena: ryo_execution_domain::domain::finish::types::Arena(&mut arena),
                    places: &mut places,
                },
                words: &mut words,
            },
        )?;
        assert_eq!(captures.len(), 1, "one real named producer frontier");
        let capture = captures.pop().unwrap().outcome?;
        file.commit_frontier(RuleIndex(0), capture.input)?;
        file.finish()?;
        let phase = publication_owner(
            &prepared,
            &services,
            std::sync::Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
        )?;
        let required = required_partitions(
            &prepared,
            RuleIndex(1),
            &corpus,
            &[],
            &[],
            &[],
            &[],
            &services,
        )?;
        let length = capture.words.0.len();
        let token = crate::application::file_dag::program::restored::validate_restored_frontier(
            &prepared,
            RuleIndex(0),
            InputNumber(0),
            Bucket(0),
            capture.bucket,
            &required,
            crate::outbound::facts::types::KeptBucket {
                words: Arc::new(KeptWords(capture.words)),
                sources: vec![crate::outbound::facts::types::BucketSource {
                    file: FileId(0),
                    start: crate::outbound::facts::types::KeptWordsStart(0),
                    length: crate::outbound::facts::types::KeptWordsLength(length),
                    proof: capture.proof,
                }],
            },
            &phase,
        )?;
        let partials = Partials::default();
        let mut project_producer = open_project(
            &prepared,
            required.clone(),
            ScopePartials::Project {
                evaluation: &partials,
                publication: &partials,
            },
            ProjectFrontiers {
                publication: &phase,
                ordinary: prepared_frontiers(
                    RuleIndex(5),
                    Arc::new([]),
                    vec![].into(),
                    Arc::clone(prepared.program()),
                    required.clone(),
                    vec![].into(),
                ),
                bound_inputs: &[],
            },
            symbols,
            &[],
            Arc::clone(&services),
        )?;
        let mut produced = project_producer.rule(
            RuleIndex(5),
            RuleDemand {
                emit: EmitDemand::Disabled,
                observations: ObservationDemand::Required,
            },
            symbols,
        )?;
        assert!(matches!(produced.emit, RootOutcome::NoQuery));
        assert_eq!(produced.observations.len(), 1);
        let crate::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome {
            outcome: observed,
            ..
        } = produced.observations.pop().unwrap();
        let ProjectObservation::Complete { table, publication } = observed else {
            panic!("real empty project member must publish successfully")
        };
        assert_eq!(table.count, RowCount(0));
        assert_eq!(
            table.kinds,
            [CellKind::File, CellKind::Number, CellKind::Number]
        );
        drop(project_producer);
        Ok(Self {
            prepared,
            phase,
            services,
            required,
            bound: vec![BoundInput {
                input: ObservationBindingId(0),
                members: Ok(vec![
                    BoundMember::Restored(token),
                    BoundMember::Current(publication),
                ]),
            }],
            refused: vec![BoundInput {
                input: ObservationBindingId(0),
                members: Err(OperatingError::WrongBinding),
            }],
        })
    }

    pub(in super::super) fn open<'a>(
        &'a self,
        rule: RuleIndex,
        partials: &'a Partials,
        symbols: &mut Symbols,
        supply: BoundSupply,
    ) -> Result<ProjectRun<'a>, OperatingError> {
        open_project(
            &self.prepared,
            self.required.clone(),
            ScopePartials::Project {
                evaluation: partials,
                publication: partials,
            },
            ProjectFrontiers {
                publication: &self.phase,
                ordinary: prepared_frontiers(
                    rule,
                    Arc::new([]),
                    vec![].into(),
                    Arc::clone(self.prepared.program()),
                    self.required.clone(),
                    vec![].into(),
                ),
                bound_inputs: if matches!(supply, BoundSupply::Refused) {
                    &self.refused
                } else {
                    &self.bound
                },
            },
            symbols,
            &[],
            Arc::clone(&self.services),
        )
    }
}
