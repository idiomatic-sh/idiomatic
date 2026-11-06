//! Test support for `cross_file`: the worlds, plans and frontier rows its cases are stated over.
#![cfg(test)]
use super::service::CrossFile;
use super::types::ProjectCrossing;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::types::FrontierDemand;
use ryo_execution::inbound::use_cases::execution::types::FrontierRequest;
use ryo_execution::inbound::use_cases::execution::types::ObservationIdentity;
use ryo_execution::inbound::use_cases::execution::types::PreparedFrontiers;
use ryo_execution::inbound::use_cases::execution::types::PreparedRequest;
use ryo_execution::inbound::use_cases::execution::types::ProjectPhase;
use ryo_execution::inbound::use_cases::execution::types::ProjectScope;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::Partial;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
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
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
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
        Arc<ryo_execution::inbound::use_cases::execution::types::PreparedExecutionProgram>,
    pub(super) request: ryo_execution::inbound::use_cases::execution::types::RequestExecution,
    pub(super) metadata: Vec<ryo_values::domain::run_metadata::types::RuleRunMetadata>,
    pub(super) ids: Vec<RuleId>,
    pub(super) paths: Vec<ryo_language::domain::files::types::FilePath>,
    pub(super) files: Vec<ryo_values::domain::files::types::CorpusInput>,
    pub(super) needs: Vec<Needs>,
    pub(super) arenas: Vec<ArenaBytes>,
    pub(super) covered: Vec<Covered>,
    pub(super) skipped: Vec<ryo_values::domain::runs::types::Skipped>,
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
        Arc::new(crate::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        )),
    )
}

pub(super) fn world_with(
    plans: Vec<Plan>,
    operators: Arc<dyn ryo_execution::inbound::use_cases::execution::types::OperatorsPort>,
) -> World {
    let applicability =
        vec![ryo_values::domain::runs::types::RuleApplicability::Always; plans.len()];
    world_with_applicability(plans, operators, &applicability)
}

pub(super) fn world_with_applicability(
    plans: Vec<Plan>,
    operators: Arc<dyn ryo_execution::inbound::use_cases::execution::types::OperatorsPort>,
    applicability: &[ryo_values::domain::runs::types::RuleApplicability],
) -> World {
    use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort as _;
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
    let matcher = crate::outbound::matching::fixtures::fake::MatchingFake
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
    let engine = ExecutionUseCaseImpl::new(
        operators,
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let program = Arc::new(
        engine
            .prepare(PreparedRequest {
                bindings: vec![Vec::new(); plans.len()],
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
                plans,
                hosted: Vec::new(),
                observations: ObservationIdentity::Independent,
            })
            .expect("prepared cross rule"),
    );
    let request = engine.begin(&program, &files);
    World {
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
            unread: &[],
            project: &world.project,
            answering: &crate::domain::render::types::Answering::Findings,
            unknown: &[],
        },
    }
}

pub(super) fn prepared<'a>(
    world: &'a World,
    rows: &[(FileId, ryo_values::domain::rows::types::Table)],
    symbols: &mut Symbols,
) -> Result<(ProjectPhase<'a>, ProjectScope, PreparedFrontiers), RuleRequestsError> {
    let phase = world.request.project_phase().expect("one project phase");
    let (scope, frontiers) =
        prepared_in(world, &phase, RuleIndex(0), &world.covered, rows, symbols)?;
    Ok((phase, scope, frontiers))
}

pub(super) fn prepared_in(
    world: &World,
    phase: &ProjectPhase<'_>,
    rule: RuleIndex,
    covered: &[Covered],
    rows: &[(FileId, Table)],
    symbols: &mut Symbols,
) -> Result<(ProjectScope, PreparedFrontiers), RuleRequestsError> {
    use ryo_execution::inbound::use_cases::execution::types::facts::{FactsPort as _, WordsToKeep};
    use ryo_values::domain::{
        records::types::{FrontierCoverage, FrontierProof},
        stored_facts::types::Bucket,
    };
    let facts_port = crate::outbound::facts::fixtures::fake::FactsFake::default();
    let mut workers = Vec::new();
    let proof = FrontierProof {
        rule: ryo_values::domain::records::types::ProducerRule(rule.0 as u32),
        input: ryo_values::domain::records::types::FrontierOrdinal(0),
        rule_digest: world.program.rule_digest(rule).expect("rule digest"),
        coverage: FrontierCoverage::Complete,
    };
    for (file, table) in rows {
        let worker = facts_port.worker().expect("facts worker");
        let mut words = Words::default();
        encode_table(table, &[], &mut words);
        let packed = pack(WordSlice(&words.0));
        worker
            .keep(
                Bucket(0),
                WordsToKeep {
                    words: &packed,
                    file: *file,
                    proof: &proof,
                },
            )
            .expect("source frame");
        workers.push(worker);
    }
    let facts = facts_port
        .finish(&mut workers.into_iter())
        .expect("facts finish");
    let project = facts_port.worker().expect("project facts");
    let scope = phase
        .project_scope(
            rule,
            &world.files,
            covered,
            &world.skipped,
            &ryo_execution::inbound::use_cases::execution::types::InapplicableFiles::default(),
        )
        .expect("project scope");
    let frontiers = world
        .request
        .prepare_frontiers(
            FrontierRequest {
                demand: FrontierDemand::Diagnostic,
                scope: &scope,
                rule,
                first: Bucket(0),
                facts: facts.as_ref(),
                project: project.as_ref(),
                project_symbols: &Symbols::default(),
            },
            symbols,
        )
        .map_err(|error| {
            ryo_values::domain::rule_requests::service::failed(&world.ids[rule.0], &error)
        })?;
    Ok((scope, frontiers))
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

impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase for ProjectCalls {
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer,
        ryo_operators::inbound::use_cases::operators::interface::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::interface::{
            OperatorCall, OperatorsUseCase,
        };
        use std::sync::atomic::Ordering;
        match &call {
            OperatorCall::ReduceCount { .. } => {
                self.reductions.fetch_add(1, Ordering::SeqCst);
            }
            OperatorCall::Aggregate { .. } => {
                self.aggregates.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
            .operators(call)
    }
}

pub(super) fn service() -> CrossFile {
    CrossFile::new(std::sync::Arc::new(
        crate::outbound::corpus::fixtures::fake::CorpusFake::default(),
    ), std::sync::Arc::new(crate::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new()))))
}
