//! Test support for `cross_file`: the worlds, plans and frontier rows its cases are stated over.
#![cfg(test)]
use super::service::CrossFile;
use super::types::ProjectCrossing;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::Partial;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution_runtime::outbound::facts::interface::{FactsFinish, FactsPort as _, FactsWorker};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_relations::domain::relations::types::Words;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::table_words::service::encode_table;
use ryo_values::domain::table_words::service::pack;
use ryo_values::domain::table_words::types::WordSlice;
use std::sync::Arc;

/// What the cross-file steps of every rule read, owned.
pub(super) struct World {
    pub(super) placed: ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes,
    pub(super) program:
        Arc<ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram>,
    pub(super) request: ryo_execution::inbound::use_cases::execution::interface::RequestExecution,
    /// Told of the rows each step of the request's rules was fed.
    pub(super) observer:
        Arc<ryo_execution_runtime::outbound::observer::fixtures::fake::ObserverFake>,
    pub(super) metadata: Vec<ryo_values::domain::run_metadata::types::RuleRunMetadata>,
    pub(super) ids: Vec<RuleId>,
    pub(super) paths: Vec<ryo_language::domain::files::types::FilePath>,
    pub(super) files: Vec<ryo_values::domain::files::types::CorpusInput>,
    pub(super) needs: Vec<Needs>,
    pub(super) arenas: Vec<ArenaBytes>,
    pub(super) covered: Vec<Covered>,
    pub(super) skipped: Vec<ryo_values::domain::runs::types::Skipped>,
    pub(super) refused: std::collections::BTreeMap<RuleId, std::collections::BTreeSet<FilePath>>,
    pub(super) project: Vec<Partial>,
}

/// A plan that counts across files the names a per-file step handed over, and emits the column `subject` of the counts.
pub(super) fn counting_plan(subject: u32, key: Vec<ColumnIndex>) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![
            PlanStep {
                operation: Operation::Project {
                    from: PlanInput::Relation(Relation::Function),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Dropped,
                },
                scope: StepScope::PerFile,
            },
            PlanStep {
                operation: Operation::GroupCount {
                    from: PlanInput::Step(StepId(0)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
                scope: StepScope::CrossFile,
            },
        ],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(subject),
            key,
            headings: Vec::new(),
        },
        frontier: vec![PlanInput::Step(StepId(0))],
        comparison_origins: Vec::new(),
    }
}

pub(super) fn world(plan: Plan) -> World {
    world_with(
        vec![plan],
        Arc::new(
            ryo_execution_runtime::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
                    std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
                ),
            ),
        ),
    )
}

pub(super) fn world_with(
    plans: Vec<Plan>,
    operators: Arc<dyn ryo_execution_runtime::outbound::operators::interface::OperatorsPort>,
) -> World {
    let applicability =
        vec![ryo_values::domain::runs::types::RuleApplicability::Always; plans.len()];
    world_with_applicability(plans, operators, &applicability)
}

pub(super) fn world_with_applicability(
    plans: Vec<Plan>,
    operators: Arc<dyn ryo_execution_runtime::outbound::operators::interface::OperatorsPort>,
    applicability: &[ryo_values::domain::runs::types::RuleApplicability],
) -> World {
    use ryo_execution::outbound::matching::interface::MatchingPort as _;
    use ryo_values::domain::{
        files::types::{CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        stored_facts::types::Digest,
    };
    let placement = MetadataUseCaseImpl.decide(
        &plans
            .iter()
            .map(
                |_| ryo_execution::inbound::use_cases::metadata::interface::RuleScope {
                    include: Vec::new(),
                    exclude: Vec::new(),
                },
            )
            .collect::<Vec<_>>(),
    );
    let matcher = ryo_execution::outbound::matching::fixtures::fake::MatchingFake
        .compile(&placement.globs)
        .expect("matcher");
    let ids = (0..plans.len())
        .map(|rule| {
            RuleId(if rule == 0 {
                "rule".into()
            } else {
                format!("rule-{rule}").into()
            })
        })
        .collect::<Vec<_>>();
    let paths = vec![FilePath("first.py".into()), FilePath("second.py".into())];
    let files = paths
        .iter()
        .cloned()
        .map(|path| {
            CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path,
                size: FileSize(0),
                modified: Nanoseconds(0),
            })
        })
        .collect::<Vec<_>>();
    let metadata = ryo_values::domain::run_metadata::service::from_plans(&plans);
    let engine = PrepareUseCaseImpl::new(
        operators,
        ryo_execution::derived(),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let program = Arc::new(
        engine
            .prepare(PrepareRequest {
                plans: PlanSource::Authored {
                    bindings: vec![Vec::new(); plans.len()],
                    plans: plans.clone(),
                },
                rules: ids
                    .iter()
                    .enumerate()
                    .map(|(rule, id)| RuleSpec {
                        id: id.clone(),
                        query: QueryBundle {
                            root: QueryDocumentSource {
                                id: QuerySourceId(id.0.clone()),
                                text: "".into(),
                            },
                            modules: Default::default(),
                        },
                        applicability: applicability[rule],
                        given: Vec::new(),
                        observations: Vec::new(),
                        spelled: RuleParameters("".into()),
                        include: Vec::new(),
                        exclude: Vec::new(),
                    })
                    .collect(),
                digests: vec![Digest([0; 16]); plans.len()],
                hosted: Vec::new(),
                observations: ObservationIdentity::Independent,
            })
            .expect("prepared cross rule"),
    );
    let observer = Arc::new(
        ryo_execution_runtime::outbound::observer::fixtures::fake::ObserverFake::default(),
    );
    let observing: Arc<dyn ryo_execution_runtime::outbound::observer::interface::ObserverPort> =
        Arc::<ryo_execution_runtime::outbound::observer::fixtures::fake::ObserverFake>::clone(
            &observer,
        );
    let request = ExecutionUseCaseImpl::new().execution(&program, &files, Some(observing));
    World {
        observer,
        placed: ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes {
            placement,
            matcher,
        },
        program,
        request,
        metadata,
        needs: vec![Needs::default(); ids.len()],
        ids,
        paths,
        files,
        arenas: Vec::new(),
        covered: Vec::new(),
        skipped: Vec::new(),
        refused: std::collections::BTreeMap::new(),
        project: Vec::new(),
    }
}

pub(super) fn crossing(world: &World) -> ProjectCrossing<'_> {
    ProjectCrossing {
        program: Arc::clone(&world.program),
        arenas: &world.arenas,
        domain: crate::domain::crossing::types::CrossingInput {
            placed: &world.placed,
            observations: &[],
            metadata: &world.metadata,
            ids: &world.ids,
            needs: &world.needs,
            paths: &world.paths,
            files: &world.files,
            covered: &world.covered,
            skipped: &world.skipped,
            refused: &world.refused,
            unread: &[],
            project: &world.project,
            answering: &crate::domain::render::types::Answering::Findings,
            unknown: &[],
        },
    }
}

/// Finished facts and a project worker beside them.
type KeptFacts = (Box<dyn FactsFinish>, Box<dyn FactsWorker>);
/// The facts of each rule's `rows` kept in that rule's own bucket (`Bucket(rule)`) for its frontier, finished as one port, with a
/// project worker beside them.
pub(super) fn kept_facts(world: &World, kept: &[(RuleIndex, &[(FileId, Table)])]) -> KeptFacts {
    use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
    use ryo_values::domain::records::types::{FrontierCoverage, FrontierProof};
    use ryo_values::domain::stored_facts::types::Bucket;
    let facts_port = ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake::default();
    let mut workers = Vec::new();
    for (rule, rows) in kept {
        // A rule the program does not declare keeps nothing: the phase refuses it by itself.
        let Ok(rule_digest) = world.program.rule_digest(*rule) else {
            continue;
        };
        let proof = FrontierProof {
            rule: ryo_values::domain::records::types::ProducerRule(
                u32::try_from(rule.0).expect("a fixture rule's number fits in u32"),
            ),
            input: ryo_values::domain::records::types::FrontierOrdinal(0),
            rule_digest,
            coverage: FrontierCoverage::Complete,
        };
        for (file, table) in *rows {
            let worker = facts_port.worker().expect("facts worker");
            let mut words = Words::default();
            encode_table(table, &[], &mut words);
            let packed = pack(WordSlice(&words.0));
            let bucket = Bucket(u32::try_from(rule.0).expect("a fixture rule numbers its bucket"));
            let kept = WordsToKeep {
                words: &packed,
                file: *file,
                proof: &proof,
            };
            worker.keep(bucket, kept).expect("source frame");
            workers.push(worker);
        }
    }
    let facts = facts_port
        .finish(&mut workers.into_iter())
        .expect("facts finish");
    let project = facts_port.worker().expect("project facts");
    (facts, project)
}

/// How many steps the observer was last told `rule` fed: the steps the phase counted for it.
pub(super) fn fed_steps(world: &World, rule: usize) -> usize {
    world.observer.reports.lock().map_or(0, |reports| {
        reports
            .iter()
            .rev()
            .find(|(reported, _)| reported.0 == rule)
            .map_or(0, |(_, fed)| fed.len())
    })
}

pub(super) fn todo(rule: usize) -> Todo {
    Todo {
        rule: RuleIndex(rule),
        id: RuleId("rule".into()),
    }
}

/// Two source files contribute three actual Function-node rows to one project count.
pub(super) fn frontier_rows() -> Vec<(FileId, Table)> {
    use ryo_relations::domain::relations::{service::node_id, types::Position};
    use ryo_values::domain::rows::types::RowCount;
    let row = |file, positions: &[u32]| {
        (
            FileId(file),
            Table {
                kinds: vec![CellKind::Node],
                count: RowCount(positions.len()),
                cells: positions
                    .iter()
                    .map(|position| Cell::Node(node_id(FileId(file), Position(*position))))
                    .collect(),
            },
        )
    };
    vec![row(0, &[1, 1]), row(1, &[2])]
}

#[derive(Debug, Default)]
pub(super) struct ProjectCalls {
    pub(super) reductions: std::sync::atomic::AtomicUsize,
    pub(super) aggregates: std::sync::atomic::AtomicUsize,
}

impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for ProjectCalls {
    fn operators(
        &self,
        mut call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        use std::sync::atomic::Ordering;
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "the audit counts reductions and group emissions; every other operator call passes through unchanged"
        )]
        match &mut call {
            OperatorCall::ReduceCount { .. } => {
                self.reductions.fetch_add(1, Ordering::SeqCst);
            }
            OperatorCall::Evaluate {
                operation:
                    ryo_values::domain::plans::types::Operation::GroupCount { .. }
                    | ryo_values::domain::plans::types::Operation::GroupDigest { .. },
                driver,
                ..
            } => {
                // One aggregate is formatted for each prepared group.
                let groups = driver.groups().map_or(0, |groups| groups.0);
                self.aggregates.fetch_add(groups, Ordering::SeqCst);
            }
            _ => {}
        }
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
            std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
        )
        .operators(call)
    }
}

pub(super) fn service() -> CrossFile {
    CrossFile::new(
        std::sync::Arc::new(crate::outbound::corpus::fixtures::fake::CorpusFake::default()),
        std::sync::Arc::new(
            ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new(),
        ),
    )
}

pub(crate) mod cases;
