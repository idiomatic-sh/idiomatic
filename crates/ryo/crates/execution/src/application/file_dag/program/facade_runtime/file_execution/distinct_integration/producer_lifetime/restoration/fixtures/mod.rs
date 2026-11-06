//! A captured file producer admitted to the real named project publication owner.
use super::super::super::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::frontiers::types::{BoundInput, BoundMember, PreparedFrontiers};
use crate::application::file_dag::program::admission::RequiredPartitions;
use crate::application::file_dag::program::facade_runtime::{
    ProjectFrontiers, ProjectRun, PublicationScope, open_project, publication_owner,
    required_partitions,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::domain::coverage::types::Partials;
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingId, ObservationBindingName,
    ObservationColumn, ObservationColumnName, ObservationName, ObservationProducer,
    ObservationReadinessLayout, PlanObservation,
};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::stored_facts::types::{Bucket, KeptWords};

#[derive(Clone, Copy, Debug)]
pub(super) enum Exit {
    Complete,
    SemanticFailure,
    RefusedBinding,
    CancelledDelivery,
}

pub(super) struct CapturedProject {
    pub(super) prepared: Arc<PreparedProgram>,
    pub(super) phase: PublicationScope,
    services: Arc<ProgramServices>,
    required: RequiredPartitions,
    bound: Vec<BoundInput>,
    refused: Vec<BoundInput>,
}

impl CapturedProject {
    pub(super) fn new(exit: Exit, symbols: &mut Symbols) -> Result<Self, OperatingError> {
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
        let mut rules = vec![rule(); 5];
        for (at, rule) in rules.iter_mut().enumerate() {
            rule.id = RuleId(format!("restored-lifecycle-{at}").into());
        }
        let producer_id = rules[0].id.clone();
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
                scope: StepScope::PerFile,
            }];
            rules[at].observations = vec![ObservationBinding {
                name: ObservationBindingName("source".into()),
                columns: columns.clone(),
                producers: vec![ObservationProducer::Output {
                    rule: producer_id.clone(),
                    output: ObservationName("captured".into()),
                }],
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
        let compiled = crate::domain::prepared_request::service::assemble(
            plans,
            vec![vec![]; rules.len()],
            &rules,
        )?;
        let derived = crate::application::file_dag::testing::program_derived_port();
        let prepared = crate::application::file_dag::program::preparation::service::prepare(
            compiled,
            &rules,
            &vec![Digest([0; 16]); rules.len()],
            &[],
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
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            ))),
            derived,
            &corpus,
        ));
        let mut intents = [FileRuleIntent::Undemanded; 5];
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
            b"",
            true,
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
            crate::application::file_dag::program::protocol::types::CaptureContext {
                file: FileId(0),
                rule_digest: Digest([0; 16]),
                worker: ryo_relations::domain::relations::types::WorkerNumber(0),
                retained: crate::domain::finish::types::Retained {
                    arena: crate::domain::finish::types::Arena(&mut arena),
                    places: &mut places,
                },
                words: &mut words,
            },
        )?;
        assert_eq!(captures.len(), 1, "one real named producer frontier");
        let capture = captures.pop().unwrap().outcome?;
        file.commit_frontier(RuleIndex(0), capture.input)?;
        file.finish()?;
        let phase = publication_owner(&prepared, &services)?;
        let required = required_partitions(&prepared, RuleIndex(1), &corpus, &[], &[], &services)?;
        let length = capture.words.0.len();
        let token = crate::application::file_dag::program::restored::validate_restored_frontier(
            &prepared,
            RuleIndex(0),
            InputNumber(0),
            Bucket(0),
            capture.bucket,
            &required,
            crate::outbound::facts::types::KeptBucket {
                words: KeptWords(capture.words),
                sources: vec![crate::outbound::facts::types::BucketSource {
                    file: FileId(0),
                    start: 0,
                    length,
                    proof: capture.proof,
                }],
            },
            &phase,
        )?;
        Ok(Self {
            prepared,
            phase,
            services,
            required,
            bound: vec![BoundInput {
                input: ObservationBindingId(0),
                members: Ok(vec![BoundMember::Restored(token)]),
            }],
            refused: vec![BoundInput {
                input: ObservationBindingId(0),
                members: Err(OperatingError::WrongBinding),
            }],
        })
    }

    pub(super) fn open<'a>(
        &'a self,
        rule: RuleIndex,
        partials: &'a Partials,
        symbols: &mut Symbols,
        refused: bool,
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
                ordinary: PreparedFrontiers {
                    producer: rule,
                    sources: Box::new([]),
                    rows: vec![],
                    program: Arc::clone(self.prepared.program()),
                    required: self.required.clone(),
                    refused: vec![],
                },
                bound_inputs: if refused { &self.refused } else { &self.bound },
            },
            symbols,
            &[],
            Arc::clone(&self.services),
        )
    }
}
