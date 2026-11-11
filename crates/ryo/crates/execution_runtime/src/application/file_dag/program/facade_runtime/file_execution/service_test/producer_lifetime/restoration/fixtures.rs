//! A captured file producer admitted to the real named project publication owner.
use super::super::super::super::*;
use super::super::super::fixtures::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::frontiers::types::{BoundInput, BoundMember};
use crate::application::file_dag::program::admission::RequiredPartitions;
use crate::application::file_dag::program::facade_runtime::project_execution::{
    ProjectRoot, evaluate_roots,
};
use crate::application::file_dag::program::facade_runtime::project_execution::{
    ProjectRun, open_project,
};
use crate::application::file_dag::program::facade_runtime::service::ProjectFrontiers;
use crate::application::file_dag::program::facade_runtime::service::PublicationScope;
use crate::application::file_dag::program::facade_runtime::service::{
    publication_owner, required_partitions,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::runtime::types::IndexLiveness;
use crate::application::file_dag::program::runtime::types::{
    AttemptCheckpoint, ConsumerReleasePoint,
};
use ryo_execution_domain::domain::coverage::types::Partials;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_store::application::file_dag::derived::owner::witness::service::{
    BackingIdentity, BackingLiveness,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Integer, Relation, Value as Cell},
};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation,
};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::plan_parts::types::{HostedNumber, InputNumber};
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::stored_facts::types::{Bucket, KeptWords};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::{RowCount, Table},
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(super) enum Exit {
    Complete,
    SemanticFailure,
    RefusedBinding,
    CancelledDelivery,
}

#[derive(Clone, Copy)]
pub(super) enum BoundSupply {
    Restored,
    Refused,
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

    pub(super) fn open<'a>(
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

pub(super) fn exercise(exit: Exit) -> Result<(), OperatingError> {
    let mut symbols = Symbols::default();
    let captured = CapturedProject::new(exit, &mut symbols)?;
    let named = captured
        .prepared
        .observation_metadata()
        .expect("actual named program");
    for (slot, _) in named.slots.iter().enumerate() {
        let union = *captured.prepared.program().observation_regions()[slot]
            .as_ref()
            .expect("valid compiled named union");
        assert_eq!(
            captured.prepared.program().regions()[union.0].phase(),
            StepScope::CrossFile,
            "restored project fixture must demand project-scoped unions, not unresolved file frontiers"
        );
    }
    let partials = Partials::default();
    let owner = Rc::clone(captured.phase.owner());
    assert!(!owner.derived().index_observation().available);
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::Required,
    };
    let runs = (1..5)
        .map(|at| {
            captured.open(
                RuleIndex(at),
                &partials,
                &mut symbols,
                if at == 4 && matches!(exit, Exit::RefusedBinding | Exit::CancelledDelivery) {
                    BoundSupply::Refused
                } else {
                    BoundSupply::Restored
                },
            )
        })
        .collect::<Vec<_>>();
    // Observe the real healthy Join, not whichever Equality happens to be
    // present in this owner. Its physical producer/consumer and population all
    // come from the same compiled rule and actual ProjectRun binding.
    let physical = |step: StepId| {
        let original = captured.prepared.program().rules()[3].original_steps[step.0 as usize]
            .as_ref()
            .expect("actual healthy project instruction");
        captured.prepared.program().logical_regions()[original.logical.0]
    };
    let producer = physical(StepId(1));
    let consumer = physical(StepId(2));
    assert_ne!(
        producer, consumer,
        "the observed backing belongs to a real upstream producer"
    );
    assert!(
        captured.prepared.program().regions()[consumer.0]
            .dependencies()
            .contains(&producer),
        "the observed Join really consumes this compiled Union"
    );
    let join_logical = captured.prepared.program().rules()[3].original_steps[2]
        .as_ref()
        .expect("actual Join metadata")
        .logical;
    let region = &captured.prepared.program().regions()[consumer.0];
    let join = region
        .instructions()
        .iter()
        .find(|instruction| instruction.logical() == join_logical)
        .expect("actual compiled Join");
    let equality = join
        .contract()
        .derived
        .slots
        .iter()
        .flatten()
        .find(|group| {
            group.primary.plan.kind == ryo_derived::domain::request::types::DerivedKind::Equality
        })
        .expect("actual Join declares Equality");
    assert_eq!(
        equality.primary.len,
        ryo_execution_domain::domain::derived_views::types::InputsUsed(1)
    );
    assert_eq!(
        region.instructions().first().map(|first| first.logical()),
        Some(join_logical),
        "observed Equality must read the region's own inputs, a real upstream publication"
    );
    assert_eq!(
        region.inputs()[equality.primary.inputs[0].0],
        ryo_execution_domain::domain::executable_program::types::InputSource::Region(producer),
        "observe the precise population demanded by the compiled Equality contract"
    );
    runs[2]
        .as_ref()
        .map_err(|error| error.clone())?
        .observe_producer_releases(RuleIndex(3), producer, &[consumer])?;
    let roots = runs
        .into_iter()
        .enumerate()
        .map(|(at, run)| ProjectRoot {
            rule: RuleIndex(at + 1),
            demand,
            run,
        })
        .collect::<Vec<_>>();
    let guards = std::cell::Cell::new(0);
    let batch = evaluate_roots(roots, &mut symbols, |rule| {
        guards.set(guards.get() + 1);
        if rule == RuleIndex(4) && matches!(exit, Exit::CancelledDelivery) {
            Err(OperatingError::NotRead)
        } else {
            Ok(())
        }
    });
    let outcomes = match batch {
        ProjectBatch::Complete(outcomes) => {
            assert!(
                !matches!(exit, Exit::CancelledDelivery),
                "cancellation must refuse its actual delivery"
            );
            outcomes
        }
        ProjectBatch::Refused { completed, error } => {
            assert!(matches!(exit, Exit::CancelledDelivery));
            assert!(matches!(error, OperatingError::NotRead));
            assert_eq!(
                completed.len(),
                3,
                "earlier delivered aliases survive later refusal"
            );
            completed
        }
    };
    let mut transferred = vec![];
    for (at, outcome) in outcomes.into_iter().enumerate() {
        let outcome = outcome?;
        match outcome.emit {
            RootOutcome::Complete(table) => transferred.push(table),
            RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: 99 })) => {
                assert_eq!(at, 3);
                assert!(matches!(exit, Exit::SemanticFailure));
            }
            RootOutcome::Failed(OperatingError::WrongBinding) => {
                assert_eq!(at, 3);
                assert!(matches!(exit, Exit::RefusedBinding));
            }
            _ => panic!("{exit:?}: actual restored delivery lost its terminal result"),
        }
    }
    assert_eq!(
        transferred[0], transferred[1],
        "duplicate named obligations deliver the same canonical rows"
    );
    assert_eq!(transferred[0].count, RowCount(2));
    let observed = captured.open(RuleIndex(3), &partials, &mut symbols, BoundSupply::Restored)?;
    let events = observed.take_producer_releases(RuleIndex(3))?;
    drop(observed);
    assert_eq!(
        events.len(),
        2,
        "one actual Join release, before and after, in its bound population"
    );
    let (before, after) = (&events[0], &events[1]);
    assert_eq!(before.consumer, consumer);
    assert_eq!(after.consumer, consumer);
    assert_eq!(before.point, ConsumerReleasePoint::Before);
    assert_eq!(after.point, ConsumerReleasePoint::After);
    assert_eq!(before.checkpoint.attempt, AttemptCheckpoint::Published);
    assert!(
        before.checkpoint.remaining.0 > 0,
        "the producer is still needed during the actual Join"
    );
    assert!(
        before.index_liveness == IndexLiveness::Alive,
        "required Equality backing is alive at its actual consumer boundary"
    );
    assert!(
        before.working_bytes.0 > 0,
        "the actual required lookup is charged while needed"
    );
    let backing = before
        .index
        .as_ref()
        .expect("backing of the qualified producer");
    let released = after
        .index
        .as_ref()
        .expect("same backing remains observable after retirement");
    assert!(
        backing.compare(released) == BackingIdentity::Same,
        "release observes the same actual allocation, not another population's index"
    );
    assert_eq!(
        after.checkpoint.remaining.0, 0,
        "this Join is the final compute consumer"
    );
    assert_eq!(after.checkpoint.attempt, AttemptCheckpoint::Terminal);
    assert!(
        after.index_liveness == IndexLiveness::Released,
        "the final compute release drops the lookup before output delivery"
    );
    assert_eq!(
        after.working_bytes.0, 0,
        "true last use releases its required charge"
    );
    assert!(backing.liveness() == BackingLiveness::Released);
    let counts = captured.phase.readiness_counts();
    let builds = owner.derived().counts();
    // The previous oracle incorrectly required a lookup to outlive the batch.
    // Returned Tables and terminal replay have no dependency on that lookup.
    assert_eq!(
        owner.derived().working_bytes(),
        0,
        "all compute lookup leases are already released"
    );
    assert!(counts.evaluations.0 > 0);
    assert!(
        builds.0 > 0,
        "the actual restored population acquires required equality backing"
    );

    assert_eq!(owner.derived().retained_bytes(), 0);

    for at in [1, 2, 4] {
        if at == 4 && matches!(exit, Exit::CancelledDelivery) {
            continue;
        }
        let mut run = captured.open(
            RuleIndex(at),
            &partials,
            &mut symbols,
            if at == 4 && matches!(exit, Exit::RefusedBinding | Exit::CancelledDelivery) {
                BoundSupply::Refused
            } else {
                BoundSupply::Restored
            },
        )?;
        for _ in 0..2 {
            let replay = run.rule(RuleIndex(at), demand, &mut symbols)?;
            match (at, exit, replay.emit) {
                (
                    4,
                    Exit::SemanticFailure,
                    RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn {
                        index: 99,
                    })),
                )
                | (4, Exit::RefusedBinding, RootOutcome::Failed(OperatingError::WrongBinding)) => {}
                (_, _, RootOutcome::Complete(table)) => assert!(table.count.0 > 0),
                _ => panic!(
                    "{exit:?}: repeated project delivery must preserve its exact terminal outcome"
                ),
            }
            assert_eq!(
                captured.phase.readiness_counts(),
                counts,
                "replay cannot schedule a terminal restored root"
            );
            assert_eq!(
                owner.derived().counts(),
                builds,
                "replay cannot reacquire a restored view"
            );
            assert_eq!(owner.derived().working_bytes(), 0);
            assert!(
                backing.liveness() == BackingLiveness::Released,
                "copied output replay must not reacquire the retired lookup"
            );
        }
    }
    captured.phase.finish()?;
    assert_eq!(owner.derived().working_bytes(), 0);
    assert_eq!(owner.derived().owned_bytes(), 0);
    assert!(
        backing.liveness() == BackingLiveness::Released,
        "phase finish cannot revive the already released compute backing"
    );
    assert_eq!(
        owner.derived().counts(),
        builds,
        "finish cannot dispatch or acquire work"
    );
    assert_eq!(
        transferred[0].count,
        RowCount(2),
        "transferred rows outlive phase retirement"
    );
    Ok(())
}
