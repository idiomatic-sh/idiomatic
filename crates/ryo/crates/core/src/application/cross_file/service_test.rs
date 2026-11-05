// the cases share one injected dependency scenario to prove retained-data loading, resident lookups, ordered output, and each required port failure; keeping the fixtures and the complete case table beside the application operation makes the same boundary visible in every case.
//! The `crossing` service's cases, on values alone.

use super::service::CrossFile;
use crate::application::run_coordination::settings::CrossingInput;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::{partials_for, placeable};
use crate::domain::crossing::types::Placeability;
use ryo_execution::inbound::use_cases::execution::{
    interface::{ExecutionRequest, ExecutionUseCase, PreparedExecution, ProjectStage},
    types::{
        FrontierDemand, FrontierRequest, ObservationIdentity, PreparedFrontiers, PreparedRequest,
        ProjectPhase, ProjectScope, PublicationSlot,
    },
    use_case::ExecutionUseCaseImpl,
};
use ryo_execution::inbound::use_cases::metadata::interface::{
    Covered, FilePartial, Lacking, Partial, RuleIndex,
};
use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Name, NamePlace, Offset, Origin, PlaceNumber, Relation, TextHash,
    Value as Cell, Words, WorkerNumber,
};
use ryo_values::domain::files::types::{CorpusEntry, FileBytes};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, RepeatedRows, StepId,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::rule_requests::errors::RulesError;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::WordSlice;
use std::sync::Arc;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// Resource substitution changes rendered source spelling.
    Substitution,
    /// An unreadable source yields no invented text.
    Unreadable,
    /// Findings whose columns do and do not say where they are.
    Placeable,
    /// Files and the project that read a relation in part which a rule trusts.
    Partials,
    /// The nodes two files handed over, counted.
    CrossCounts,
    /// A rule the plans lack.
    CrossNoRule,
    /// A cross-file step that reads what nobody handed over.
    CrossNothingHanded,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Up to three numbers a row answers; a place a row has no number for holds zero.
type Value = [usize; 3];
type Failure = RulesError;

/// What the cross-file steps of every rule read, owned.
struct World {
    placed: ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes,
    program: Arc<ryo_execution::inbound::use_cases::execution::types::PreparedProgram>,
    request: ryo_execution::inbound::use_cases::execution::types::RequestExecution,
    metadata: Vec<ryo_values::domain::run_metadata::types::RuleMetadata>,
    ids: Vec<RuleId>,
    paths: Vec<ryo_language::domain::files::types::FilePath>,
    files: Vec<ryo_values::domain::files::types::CorpusInput>,
    needs: Vec<Needs>,
    arenas: Vec<ArenaBytes>,
    covered: Vec<Covered>,
    skipped: Vec<ryo_values::domain::runs::types::Skipped>,
    project: Vec<Partial>,
}

/// A plan that counts across files the names a per-file step handed over, and emits the column `subject` of the counts.
fn counting_plan(subject: u32, key: Vec<ColumnIndex>) -> Plan {
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

fn world(plan: Plan) -> World {
    world_with(
        vec![plan],
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
    )
}
fn world_with(
    plans: Vec<Plan>,
    operators: Arc<dyn ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase>,
) -> World {
    let applicability =
        vec![ryo_values::domain::runs::types::RuleApplicability::Always; plans.len()];
    world_with_applicability(plans, operators, &applicability)
}
fn world_with_applicability(
    plans: Vec<Plan>,
    operators: Arc<dyn ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase>,
    applicability: &[ryo_values::domain::runs::types::RuleApplicability],
) -> World {
    use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort as _;
    use ryo_values::domain::{
        files::types::{CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
        program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
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

fn crossing(world: &World) -> CrossingInput<'_> {
    CrossingInput {
        program: Arc::clone(&world.program),
        services: &world.request,
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
            project: &world.project,
            answering: &crate::domain::render::types::Answering::Findings,
            unknown: &[],
        },
    }
}

fn prepared<'a>(
    world: &'a World,
    rows: &[(FileId, ryo_values::domain::rows::types::Table)],
    symbols: &mut Symbols,
) -> Result<(ProjectPhase<'a>, ProjectScope, PreparedFrontiers), RulesError> {
    let phase = world.request.project_phase().expect("one project phase");
    let (scope, frontiers) =
        prepared_in(world, &phase, RuleIndex(0), &world.covered, rows, symbols)?;
    Ok((phase, scope, frontiers))
}
fn prepared_in(
    world: &World,
    phase: &ProjectPhase<'_>,
    rule: RuleIndex,
    covered: &[Covered],
    rows: &[(FileId, Table)],
    symbols: &mut Symbols,
) -> Result<(ProjectScope, PreparedFrontiers), RulesError> {
    use ryo_execution::inbound::use_cases::execution::types::facts::{FactsPort as _, WordsToKeep};
    use ryo_values::domain::{
        records::types::{FrontierCoverage, FrontierProof},
        stored_facts::types::Bucket,
    };
    let facts_port = crate::outbound::facts::fixtures::fake::FactsFake::default();
    let mut workers = Vec::new();
    let proof = FrontierProof {
        version: 1,
        rule: rule.0 as u32,
        input: 0,
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
        .project_scope(rule, &world.files, covered, &world.skipped)
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
        .map_err(|error| crate::domain::rules::service::failed(&world.ids[rule.0], &error))?;
    Ok((scope, frontiers))
}

fn todo(rule: usize) -> Todo {
    Todo {
        rule: RuleIndex(rule),
        id: RuleId("rule".into()),
    }
}

fn partial(relation: Relation) -> Partial {
    Partial {
        relation,
        cause: PartialCause("a construct the reader cannot represent".into()),
        lacking: Lacking::Any,
        at: None,
    }
}

fn name(hash: u64, place: u32) -> Cell {
    Cell::Name(Name {
        hash: TextHash(hash),
        place: PlaceNumber(place),
    })
}

/// Two source files contribute three actual Function-node rows to one project count.
fn frontier_rows() -> Vec<(FileId, Table)> {
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

/// The file of `partial`, as a number: the repository as a whole is the number past every file, as the store writes it; 0
/// when there is no such partial.
fn file_number(partial: Option<&FilePartial>) -> usize {
    partial.map_or(0, |each| {
        usize::try_from(each.file.map_or(u32::MAX, |file| file.0)).unwrap_or(0)
    })
}

/// The partials of a rule that reads what the project wrote: the files that cover it, in order.
fn partials_found() -> Value {
    let mut rules = world(counting_plan(1, Vec::new()));
    rules.needs = vec![Needs {
        cross: vec![RelationRead {
            relation: Relation::Function,
            kinds: Vec::new(),
        }],
        ..Needs::default()
    }];
    rules.project = vec![
        partial(Relation::Function),
        partial(Relation::PackageDependency),
    ];
    rules.covered = vec![
        Covered {
            file: FileId(2),
            rules: vec![RuleIndex(0)],
            partial: vec![partial(Relation::Function)],
        },
        Covered {
            file: FileId(3),
            rules: vec![RuleIndex(1)],
            partial: vec![partial(Relation::Function)],
        },
    ];
    let found = partials_for(&crossing(&rules), RuleIndex(0)).entries;
    [
        found.len(),
        file_number(found.first()),
        file_number(found.get(1)),
    ]
}

fn placing_outcomes() -> Value {
    let table = Table {
        kinds: vec![CellKind::Symbol, CellKind::Number],
        ..Table::default()
    };
    let says = |emitted, key| {
        usize::from(placeable(&counting_plan(emitted, key).emit, &table) == Placeability::Placeable)
    };
    [
        says(0, Vec::new()),
        says(1, Vec::new()),
        says(1, vec![ColumnIndex(0)]),
    ]
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::Substitution => [
            usize::from(source_text(b"one", false) == "one"),
            usize::from(source_text(b"two", false) == "two"),
            0,
        ],
        Scenario::Unreadable => [usize::from(source_text(b"one", true).is_empty()), 0, 0],
        Scenario::Placeable => placing_outcomes(),
        Scenario::Partials => partials_found(),
        Scenario::CrossCounts => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let (_phase, scope, frontier) = prepared(&rules, &frontier_rows(), &mut cross)?;
            let done =
                service().evaluate(&crossing(&rules), &todo(0), frontier, &mut cross, &scope)?;
            [
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                done.fed.len(),
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.cells.len()),
            ]
        }
        Scenario::CrossNoRule => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let (_phase, scope, frontier) = prepared(&rules, &frontier_rows(), &mut cross)?;
            let done =
                service().evaluate(&crossing(&rules), &todo(3), frontier, &mut cross, &scope)?;
            [
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                done.fed.len(),
                0,
            ]
        }
        Scenario::CrossNothingHanded => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let (_phase, scope, frontier) = prepared(&rules, &[], &mut cross)?;
            let done =
                service().evaluate(&crossing(&rules), &todo(0), frontier, &mut cross, &scope)?;
            [
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                0,
                0,
            ]
        }
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("substituting_the_injected_corpus_changes_source_spelling"),
        word: Word::Succeeds,
        input: || Scenario::Substitution,
        expect: Expect::Value([1, 1, 0]),
    },
    Case {
        name: CaseName("failed_source_reads_never_invent_a_spelling"),
        word: Word::Succeeds,
        input: || Scenario::Unreadable,
        expect: Expect::Value([1, 0, 0]),
    },
    Case {
        name: CaseName("a_finding_is_placeable_by_a_name_or_a_key_that_says_where_it_is"),
        word: Word::Succeeds,
        input: || Scenario::Placeable,
        expect: Expect::Value([1, 0, 1]),
    },
    Case {
        name: CaseName(
            "the_partial_reads_of_the_project_and_the_files_that_govern_a_rule_bear_on_it",
        ),
        word: Word::Succeeds,
        input: || Scenario::Partials,
        expect: Expect::Value([2, 4_294_967_295, 2]),
    },
    Case {
        name: CaseName(
            "the_nodes_two_files_handed_over_are_counted_into_two_findings_of_five_cells",
        ),
        word: Word::Succeeds,
        input: || Scenario::CrossCounts,
        expect: Expect::Value([2, 2, 10]),
    },
    Case {
        name: CaseName("an_unprepared_rule_refuses_the_project_scope"),
        word: Word::Refuses,
        input: || Scenario::CrossNoRule,
        expect: Expect::Refusal(|error| {
            if matches!(error, RulesError::Failed { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_step_that_reads_what_nobody_handed_over_fails_the_rule"),
        word: Word::Refuses,
        input: || Scenario::CrossNothingHanded,
        expect: Expect::Refusal(|error| {
            if matches!(error, RulesError::Failed { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

fn service() -> CrossFile {
    CrossFile::new(std::sync::Arc::new(
        crate::outbound::corpus::fixtures::fake::CorpusFake::default(),
    ), std::sync::Arc::new(crate::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new()))))
}
fn source_text(bytes: &[u8], fails: bool) -> String {
    use crate::outbound::corpus::fixtures::fake::CorpusFake;
    use ryo_values::domain::files::types::CorpusFile;
    use ryo_values::domain::files::types::{FileSize, Nanoseconds};
    let corpus = CorpusFake {
        files: if fails {
            Vec::new()
        } else {
            vec![CorpusFile {
                executable: Default::default(),
                path: FilePath("source".into()),
                bytes: FileBytes(bytes.to_vec()),
            }]
        },
        fails,
        reads: std::sync::Arc::default(),
        outside: Vec::new(),
    };
    let service = CrossFile::new(std::sync::Arc::new(corpus), std::sync::Arc::new(crate::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new()))));
    let files = [CorpusEntry {
        executable: Default::default(),
        path: FilePath("source".into()),
        size: FileSize(3),
        modified: Nanoseconds(0),
    }]
    .map(ryo_values::domain::files::types::CorpusInput::Available);
    let mut symbols = Symbols::default();
    places_mut(&mut symbols).push(NamePlace {
        origin: Origin::Source,
        file: FileId(0),
        start: Offset(0),
        end: Offset(3),
    });
    let resolving = crate::domain::finish::types::Resolving {
        unknown: &[],
        paths: &[],
        arenas: crate::domain::finish::types::Arenas(&[]),
        cross: &symbols,
    };
    let mut text = ryo_values::domain::records::types::RenderedText::default();
    let shown = service.show_value(
        &mut text,
        name(1, 0),
        &files,
        &resolving,
        &mut crate::domain::finish::types::Loaded::default(),
    );
    match shown.map(|kept| {
        ryo_values::domain::records::service::lent(
            ryo_values::domain::records::types::RowsTextView(&text),
            kept,
        )
    }) {
        Some(ryo_values::domain::records::types::Cell::Text(piece)) => piece.0.to_owned(),
        Some(
            ryo_values::domain::records::types::Cell::File(_)
            | ryo_values::domain::records::types::Cell::Number(_)
            | ryo_values::domain::records::types::Cell::Span(_),
        )
        | None => String::new(),
    }
}

use ryo_values::domain::rows::types::Table;

use ryo_execution::inbound::use_cases::metadata::{
    interface::MetadataUseCase, use_case::MetadataUseCaseImpl,
};

#[test]
fn current_project_roots_keep_the_original_two_group_four_cell_result() {
    use ryo_execution::inbound::use_cases::execution::{
        interface::ProjectExecution,
        types::{EmitDemand, ObservationDemand, Partials, ProjectInput, RootOutcome, RuleDemand},
    };
    let world = world(counting_plan(1, Vec::new()));
    let mut symbols = Symbols::default();
    let (_phase, scope, ordinary) =
        prepared(&world, &frontier_rows(), &mut symbols).expect("actual source-bound frontier");
    let partials = Partials::default();
    let mut run = world
        .request
        .open_project(
            ProjectInput {
                scope: &scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &world.arenas,
            },
            &mut symbols,
        )
        .expect("project");
    let done = run
        .rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::Required,
            },
            &mut symbols,
        )
        .expect("rule");
    let RootOutcome::Complete(table) = done.emit else {
        panic!("complete group root")
    };
    assert_eq!(
        [table.count.0, done.counts.fed.len(), table.cells.len()],
        [2, 2, 4]
    );
    run.finish();
}

#[test]
fn rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics() {
    use ryo_values::domain::runs::types::{SkipReason, Skipped};
    let mut world = world(counting_plan(0, Vec::new()));
    let path = ryo_language::domain::files::types::FilePath("rejected.py".into());
    world.paths = vec![path.clone()];
    world.needs[0].cross.push(RelationRead {
        relation: Relation::Function,
        kinds: vec![],
    });
    let skipped = [Skipped {
        rule: world.ids[0].clone(),
        path,
        reason: SkipReason::SyntaxRejected,
    }];
    let mut input = crossing(&world);
    input.domain.skipped = &skipped;
    let partials = partials_for(&input, RuleIndex(0));
    assert!(partials.entries.is_empty(), "no provider partial invented");
    assert_eq!(
        partials.unavailable_files,
        [FileId(0)],
        "skipped source remains unavailable for absence checks"
    );
}

#[test]
fn project_evaluation_rejects_frontiers_from_another_request() {
    use ryo_execution::inbound::use_cases::execution::types::OperatingError;
    let rules = world(counting_plan(1, Vec::new()));
    let mut symbols = Symbols::default();
    let (_phase, _scope, frontier) = prepared(&rules, &frontier_rows(), &mut symbols)
        .expect("frontiers admitted by the original request");
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let next = engine.begin(&rules.program, &rules.files);
    let next_phase = next.project_phase().expect("independent project phase");
    let foreign = next_phase
        .project_scope(RuleIndex(0), &rules.files, &rules.covered, &[])
        .expect("independent request over the same program and corpus");
    let result = service().evaluate(
        &crossing(&rules),
        &todo(0),
        frontier,
        &mut symbols,
        &foreign,
    );
    match result {
        Err(error) => assert_eq!(
            error,
            crate::domain::rules::service::failed(&rules.ids[0], &OperatingError::NotRead)
        ),
        Ok(_) => panic!("an unrelated request scope cannot admit the original frontiers"),
    }
}

#[derive(Debug, Default)]
struct ProjectCalls {
    reductions: std::sync::atomic::AtomicUsize,
    aggregates: std::sync::atomic::AtomicUsize,
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

#[test]
fn ordinary_project_rules_share_one_preparation_and_reject_another_generation() {
    use ryo_execution::inbound::use_cases::execution::types::OperatingError;
    use std::sync::atomic::Ordering;
    for order in [[0, 1], [1, 0]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, Vec::new());
        let mut rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let mut first_rendered = None;
        for rule in order {
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                &frontier_rows(),
                &mut symbols,
            )
            .unwrap();
            let result = service()
                .evaluate(
                    &crossing(&rules),
                    &Todo {
                        rule: RuleIndex(rule),
                        id: rules.ids[rule].clone(),
                    },
                    frontier,
                    &mut symbols,
                    &scope,
                )
                .unwrap();
            let rendered = result.rendered.expect("actual core project rendering");
            assert_eq!(
                [rendered.order.len(), result.fed.len(), rendered.cells.len()],
                [2, 2, 4]
            );
            if let Some(first) = &first_rendered {
                assert_eq!(&rendered, first);
            } else {
                first_rendered = Some(rendered);
            }
            assert_eq!(
                calls.reductions.load(Ordering::SeqCst),
                1,
                "one real grouping setup across both logical roots"
            );
            assert_eq!(
                calls.aggregates.load(Ordering::SeqCst),
                2,
                "the two authored node groups aggregate once each"
            );
        }
        // A changed physical coverage declaration cannot consume a cached publication.
        let changed = [Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0)],
            partial: vec![],
        }];
        let different = phase
            .project_scope(RuleIndex(1), &rules.files, &changed, &[])
            .unwrap();
        let (_, old_frontier) = prepared_in(
            &rules,
            &phase,
            RuleIndex(1),
            &rules.covered,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        let result = service().evaluate(
            &crossing(&rules),
            &Todo {
                rule: RuleIndex(1),
                id: rules.ids[1].clone(),
            },
            old_frontier,
            &mut symbols,
            &different,
        );
        assert!(
            matches!(result, Err(error) if error == crate::domain::rules::service::failed(&rules.ids[1], &OperatingError::NotRead))
        );
        drop(different);
        let (changed_scope, changed_frontier) = prepared_in(
            &rules,
            &phase,
            RuleIndex(1),
            &changed,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        let result = service().evaluate(
            &crossing(&rules),
            &Todo {
                rule: RuleIndex(1),
                id: rules.ids[1].clone(),
            },
            changed_frontier,
            &mut symbols,
            &changed_scope,
        );
        assert!(
            matches!(result, Err(error) if error == crate::domain::rules::service::failed(
                &rules.ids[1], &OperatingError::WrongOperation,
            )),
            "one interpreter cannot replay a different declared physical partition"
        );
        drop(changed_scope);
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 2);
        let (_, foreign) = prepared_in(
            &rules,
            &phase,
            RuleIndex(0),
            &rules.covered,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        phase.finish().unwrap();

        let engine = ExecutionUseCaseImpl::new(
            calls.clone(),
            Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
        );
        rules.request = engine.begin(&rules.program, &rules.files);
        let phase = rules.request.project_phase().unwrap();
        let scope = phase
            .project_scope(RuleIndex(0), &rules.files, &rules.covered, &[])
            .unwrap();
        let failed = service().evaluate(&crossing(&rules), &todo(0), foreign, &mut symbols, &scope);
        assert!(
            matches!(failed, Err(error) if error == crate::domain::rules::service::failed(&rules.ids[0], &OperatingError::NotRead))
        );
        drop(scope);
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        let (scope, frontier) = prepared_in(
            &rules,
            &phase,
            RuleIndex(0),
            &rules.covered,
            &frontier_rows(),
            &mut symbols,
        )
        .unwrap();
        let result = service()
            .evaluate(&crossing(&rules), &todo(0), frontier, &mut symbols, &scope)
            .unwrap();
        assert_eq!(result.rendered, first_rendered);
        assert_eq!(
            calls.reductions.load(Ordering::SeqCst),
            2,
            "a new request requires its own grouping payload"
        );
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 4);
        drop(scope);
        phase.finish().unwrap();
    }
}

#[test]
fn ordinary_phase_keeps_valid_rule_populations_and_partiality_separate() {
    use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
    use std::sync::atomic::Ordering;
    for order in [[0, 1, 0, 1], [1, 0, 1, 0]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let mut rules = world_with_applicability(
            vec![plan.clone(), plan],
            calls.clone(),
            &[RuleApplicability::AcceptedSyntax, RuleApplicability::Always],
        );
        rules.skipped.push(Skipped {
            rule: rules.ids[0].clone(),
            path: rules.paths[1].clone(),
            reason: SkipReason::SyntaxRejected,
        });
        rules.covered = vec![
            Covered {
                file: FileId(0),
                rules: vec![RuleIndex(0), RuleIndex(1)],
                partial: vec![],
            },
            Covered {
                file: FileId(1),
                rules: vec![RuleIndex(1)],
                partial: vec![],
            },
        ];
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let rows = frontier_rows();
        let mut results = vec![None, None];
        for (at, rule) in order.into_iter().enumerate() {
            // Both scopes receive the same complete request table. Only A has
            // positive skip provenance; B must retain the second file's rows.
            let source = if rule == 0 { &rows[..1] } else { &rows[..] };
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                source,
                &mut symbols,
            )
            .unwrap();
            let done = service()
                .evaluate(
                    &crossing(&rules),
                    &Todo {
                        rule: RuleIndex(rule),
                        id: rules.ids[rule].clone(),
                    },
                    frontier,
                    &mut symbols,
                    &scope,
                )
                .unwrap();
            let rendered = done
                .rendered
                .expect("valid population renders through the actual core route");
            assert_eq!(
                [rendered.order.len(), rendered.cells.len()],
                if rule == 0 { [1, 2] } else { [2, 4] }
            );
            assert_eq!(done.unknowns.len(), usize::from(rule == 0));
            if let Some(previous) = &results[rule] {
                assert_eq!(&rendered, previous);
            } else {
                results[rule] = Some(rendered);
            }
            if at >= 1 {
                assert_eq!(calls.reductions.load(Ordering::SeqCst), 2);
                assert_eq!(calls.aggregates.load(Ordering::SeqCst), 3);
            }
        }
        phase.finish().unwrap();
    }
}

#[test]
fn equal_policies_do_not_alias_distinct_admitted_frontier_payloads() {
    use ryo_relations::domain::relations::{service::node_id, types::Position};
    use std::sync::atomic::Ordering;
    for order in [[0, 1, 0], [1, 0, 1]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let original = frontier_rows();
        let mut changed = original.clone();
        for cell in &mut changed[0].1.cells {
            *cell = Cell::Node(node_id(FileId(0), Position(9)));
        }
        let mut results = vec![None, None];
        for (at, rule) in order.into_iter().enumerate() {
            let source = if rule == 0 { &original } else { &changed };
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                source,
                &mut symbols,
            )
            .unwrap();
            let raw = crate::application::run_coordination::crossing::cross_rule_with_inputs(
                &crossing(&rules),
                &Todo {
                    rule: RuleIndex(rule),
                    id: rules.ids[rule].clone(),
                },
                frontier,
                &[],
                &mut symbols,
                &scope,
            )
            .unwrap();
            let table = raw.roots.emit.expect("the actual prepared population");
            let first = if rule == 0 { 1 } else { 9 };
            assert_eq!(
                table.cells,
                [
                    Cell::Node(node_id(FileId(0), Position(first))),
                    Cell::Number(ryo_relations::domain::relations::types::Integer(2)),
                    Cell::Node(node_id(FileId(1), Position(2))),
                    Cell::Number(ryo_relations::domain::relations::types::Integer(1)),
                ]
            );
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                source,
                &mut symbols,
            )
            .unwrap();
            let done = service()
                .evaluate(
                    &crossing(&rules),
                    &Todo {
                        rule: RuleIndex(rule),
                        id: rules.ids[rule].clone(),
                    },
                    frontier,
                    &mut symbols,
                    &scope,
                )
                .unwrap();
            assert!(done.unknowns.is_empty());
            let rendered = done.rendered.unwrap();
            assert_eq!([rendered.order.len(), rendered.cells.len()], [2, 4]);
            if let Some(previous) = &results[rule] {
                assert_eq!(&rendered, previous);
            } else {
                results[rule] = Some(rendered);
            }
            if at >= 1 {
                assert_eq!(calls.reductions.load(Ordering::SeqCst), 2);
                assert_eq!(calls.aggregates.load(Ordering::SeqCst), 4);
            }
        }
        phase.finish().unwrap();
    }
}

#[test]
fn many_distinct_project_populations_allocate_only_reached_region_state() {
    use ryo_values::domain::plan_parts::types::{Comparison, Operand, QueryLiteral, Test};
    use std::sync::atomic::Ordering;
    const POPULATIONS: usize = 8;
    let calls = Arc::new(ProjectCalls::default());
    let mut plans = Vec::new();
    for seed in 0..96 {
        let mut plan = counting_plan(1, vec![]);
        if seed != 0 {
            // Separate dormant regions, with a semantically valid literal predicate.
            plan.steps.push(PlanStep {
                scope: StepScope::CrossFile,
                operation: Operation::Select {
                    from: PlanInput::Step(StepId(1)),
                    comparisons: vec![Comparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Literal(QueryLiteral::Number(
                            ryo_relations::domain::relations::types::Integer(seed),
                        )),
                        origin: None,
                    }],
                    kept: vec![],
                },
            });
            plan.emit.step = StepId(2);
        }
        plans.push(plan);
    }
    let rules = world_with(plans, calls.clone());
    let phase = rules.request.project_phase().unwrap();
    assert_eq!(phase.state_counts(), Default::default());
    let mut symbols = Symbols::default();
    for population in 0..POPULATIONS {
        let mut rows = frontier_rows();
        // Distinct legitimate transport payloads force distinct input populations.
        let repeated = rows[0].1.cells[0];
        rows[0].1.cells.push(repeated);
        rows[0].1.count.0 += 1;
        for _ in 0..population {
            rows[0].1.cells.push(repeated);
            rows[0].1.count.0 += 1;
        }
        let (scope, frontier) = prepared_in(
            &rules,
            &phase,
            RuleIndex(0),
            &rules.covered,
            &rows,
            &mut symbols,
        )
        .unwrap();
        let result = service()
            .evaluate(&crossing(&rules), &todo(0), frontier, &mut symbols, &scope)
            .unwrap();
        assert_eq!(result.rendered.as_ref().unwrap().order.len(), 2);
        let state = phase.state_counts();
        assert_eq!(state.populations.0, population + 1);
        assert_eq!(
            state.slots.0,
            population + 1,
            "only the demanded GroupCount region owns mutable state"
        );
        assert_eq!(
            state.instruction_counters.0,
            population + 1,
            "dormant authored selectors allocate no counter arrays"
        );
        assert!(state.dormant_obligations.0 <= population + 1);
    }
    assert_eq!(calls.reductions.load(Ordering::SeqCst), POPULATIONS);
    assert_eq!(calls.aggregates.load(Ordering::SeqCst), POPULATIONS * 2);
    phase.finish().unwrap();
}

#[test]
fn project_root_set_prepares_shared_publications_once_before_ordered_delivery() {
    use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
    use std::sync::atomic::Ordering;
    for order in [[0, 1, 0], [1, 0, 1]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let rules = world_with(vec![plan.clone(), plan], calls.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let mut scopes = Vec::new();
        let mut frontiers = Vec::new();
        let mut todos = Vec::new();
        for rule in order {
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                &frontier_rows(),
                &mut symbols,
            )
            .unwrap();
            scopes.push(scope);
            frontiers.push(frontier);
            todos.push(Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            });
        }
        let tasks = todos
            .iter()
            .zip(&scopes)
            .zip(frontiers)
            .map(|((todo, scope), frontier)| super::types::ProjectTask {
                todo,
                scope,
                frontier,
                bound: &[],
                slot: PublicationSlot::Unused,
            })
            .collect::<Vec<_>>();
        let results = service().evaluate_batch(&crossing(&rules), tasks, &phase, &mut symbols);
        let mut previous = None;
        for result in results {
            let result = result.unwrap().crossed;
            assert!(result.unknowns.is_empty());
            let rendered = result.rendered.unwrap();
            assert_eq!([rendered.order.len(), rendered.cells.len()], [2, 4]);
            if let Some(previous) = &previous {
                assert_eq!(&rendered, previous);
            }
            previous = Some(rendered);
        }
        let counts = phase.readiness_counts();
        assert_eq!(
            counts.preparations.0, 1,
            "one canonical root, including duplicate logical roots"
        );
        assert_eq!(counts.evaluations.0, 1);
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 2);
        assert_eq!(phase.state_counts().slots.0, 1);
        let before_replay = phase.readiness_counts();
        let mut again_scopes = Vec::new();
        let mut again_frontiers = Vec::new();
        for todo in &todos {
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                todo.rule,
                &rules.covered,
                &frontier_rows(),
                &mut symbols,
            )
            .unwrap();
            again_scopes.push(scope);
            again_frontiers.push(frontier);
        }
        let again = todos
            .iter()
            .zip(&again_scopes)
            .zip(again_frontiers)
            .map(|((todo, scope), frontier)| super::types::ProjectTask {
                todo,
                scope,
                frontier,
                bound: &[],
                slot: PublicationSlot::Unused,
            })
            .collect();
        for result in service().evaluate_batch(&crossing(&rules), again, &phase, &mut symbols) {
            assert_eq!(result.unwrap().crossed.rendered.as_ref(), previous.as_ref());
        }
        assert_eq!(
            phase.readiness_counts(),
            before_replay,
            "delivery adds no hidden fallback polls"
        );
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 1);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 2);
        drop(again_scopes);
        drop(scopes);
        phase.finish().unwrap();
    }
}

#[test]
fn project_root_set_preserves_distinct_actual_population_policy_in_both_orders() {
    use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
    use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
    use std::sync::atomic::Ordering;
    for order in [[0, 1], [1, 0]] {
        let calls = Arc::new(ProjectCalls::default());
        let plan = counting_plan(1, vec![]);
        let mut rules = world_with_applicability(
            vec![plan.clone(), plan],
            calls.clone(),
            &[RuleApplicability::AcceptedSyntax, RuleApplicability::Always],
        );
        rules.skipped.push(Skipped {
            rule: rules.ids[0].clone(),
            path: rules.paths[1].clone(),
            reason: SkipReason::SyntaxRejected,
        });
        rules.covered = vec![
            Covered {
                file: FileId(0),
                rules: vec![RuleIndex(0), RuleIndex(1)],
                partial: vec![],
            },
            Covered {
                file: FileId(1),
                rules: vec![RuleIndex(1)],
                partial: vec![],
            },
        ];
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let rows = frontier_rows();
        let mut scopes = Vec::new();
        let mut frontiers = Vec::new();
        let mut todos = Vec::new();
        for rule in order {
            let source = if rule == 0 { &rows[..1] } else { &rows[..] };
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                source,
                &mut symbols,
            )
            .unwrap();
            scopes.push(scope);
            frontiers.push(frontier);
            todos.push(Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            });
        }
        let tasks = todos
            .iter()
            .zip(&scopes)
            .zip(frontiers)
            .map(|((todo, scope), frontier)| super::types::ProjectTask {
                todo,
                scope,
                frontier,
                bound: &[],
                slot: PublicationSlot::Unused,
            })
            .collect();
        let results = service().evaluate_batch(&crossing(&rules), tasks, &phase, &mut symbols);
        for (rule, result) in order.into_iter().zip(results) {
            let result = result.unwrap().crossed;
            assert_eq!(result.unknowns.len(), usize::from(rule == 0));
            let rendered = result.rendered.unwrap();
            assert_eq!(
                [rendered.order.len(), rendered.cells.len()],
                if rule == 0 { [1, 2] } else { [2, 4] }
            );
        }
        assert_eq!(phase.readiness_counts().evaluations.0, 2);
        assert_eq!(phase.state_counts().populations.0, 2);
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 2);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 3);
        drop(scopes);
        phase.finish().unwrap();
    }
}
