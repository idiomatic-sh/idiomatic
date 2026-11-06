// ignore: idiomatic.arch/file-length - the file and project measurements share one private subscriber and one heap-window state; splitting would widen that state to a second module.
//! Real public file opening, observed without allocating in the activation subscriber.
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionRequest, ExecutionUseCase, FileExecution, ProjectRootExecution, ProjectStage,
    },
    types::*,
    use_case::ExecutionUseCaseImpl,
};
use ryo_language::domain::{
    elements::types::Elements, files::types::FilePath, symbols::types::Symbols,
};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, ColumnsView, FileId, Relation},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{
        ColumnIndex, Comparison, Direction, Input, JoinColumn, LiteralText, Operand, QueryLiteral,
        Side, StepId, Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::{Bucket, Digest},
};
use std::{
    cell::Cell,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Sample {
    /// Rule-relation lookup, relation visit, graph probe, selector-range lookup,
    /// selector visit, comparison visit, repetition visit, fanout visit.
    pub work: [u64; 8],
    /// Actual visited rule indices in relation, graph and selector lookups.
    pub rule_sums: [u64; 3],
    pub graph_hits: u64,
    pub blocks: u64,
    pub bytes: u64,
}
/// Per-rule accesses to private prepared metadata, counted at their only accessors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Access {
    /// Single-rule metadata reads.
    pub reads: u64,
    /// Sum of the rule indices those reads named.
    pub read_rules: u64,
    /// Rules visited by scans over the whole prepared rule table.
    pub scanned: u64,
}
#[derive(Default)]
struct State {
    active: bool,
    starts: usize,
    ends: usize,
    before: (u64, u64),
    sample: Sample,
    access: Access,
}
// Field values borrow an event only while Visit runs; the event name is copied
// to a small ordinal instead of storing an event-owned string.
#[derive(Default)]
struct EventFields {
    event: u8,
    rule: u64,
}
impl tracing::field::Visit for EventFields {
    fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "event" {
            self.event = match value {
                "begin" => 1,
                "end" => 2,
                "relation_rule" => 3,
                "relation_visit" => 4,
                "graph_rule" => 5,
                "selector_rule" => 6,
                "selector" => 7,
                "candidate" => 8,
                "repetition" => 9,
                "fanout" => 10,
                "graph_hit" => 11,
                "rule_metadata" => 12,
                "rule_scan" => 13,
                _ => 0,
            };
        }
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "rule" {
            self.rule = value;
        }
    }
}
#[derive(Clone, Default)]
struct Activation(Arc<Mutex<State>>);
impl tracing::Subscriber for Activation {
    fn register_callsite(
        &self,
        _: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        tracing::subscriber::Interest::sometimes()
    }
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "ryo::diagnostic::activation"
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = EventFields::default();
        event.record(&mut fields);
        let mut state = self.0.lock().unwrap();
        if fields.event == 1 {
            assert!(!state.active);
            let heap = dhat::HeapStats::get();
            state.before = (heap.total_blocks, heap.total_bytes);
            state.sample = Sample::default();
            state.access = Access::default();
            state.starts += 1;
            state.active = true;
        } else if fields.event == 2 {
            assert!(state.active);
            let heap = dhat::HeapStats::get();
            state.sample.blocks = heap.total_blocks - state.before.0;
            state.sample.bytes = heap.total_bytes - state.before.1;
            state.ends += 1;
            state.active = false;
        } else if state.active && fields.event == 12 {
            state.access.reads += 1;
            state.access.read_rules += fields.rule;
        } else if state.active && fields.event == 13 {
            state.access.scanned += 1;
        } else if state.active && fields.event == 11 {
            state.sample.graph_hits += 1;
        } else if state.active && (3..=10).contains(&fields.event) {
            state.sample.work[usize::from(fields.event - 3)] += 1;
            match fields.event {
                3 => state.sample.rule_sums[0] += fields.rule,
                5 => {
                    state.sample.rule_sums[1] += fields.rule;
                }
                6 => state.sample.rule_sums[2] += fields.rule,
                _ => {}
            }
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

fn plan(index: usize) -> Plan {
    let node = ColumnIndex(0);
    let operation = if index % 2 == 1 {
        Operation::Axis {
            from: Input::Relation(Relation::Function),
            edges: Input::Relation(Relation::SyntaxChild),
            domain: Input::Relation(Relation::SyntaxTree),
            direction: Direction::Descendant,
            id: node,
            parent: node,
            child: ColumnIndex(1),
            domain_id: node,
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![JoinColumn {
                side: Side::Left,
                column: node,
            }],
        }
    } else {
        let name = ColumnIndex(
            u32::try_from(
                kinds(Relation::Function)
                    .iter()
                    .position(|kind| *kind == CellKind::Symbol)
                    .unwrap(),
            )
            .unwrap(),
        );
        Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(name),
                right: Operand::Literal(QueryLiteral::Text(LiteralText(
                    format!("{index:08}").into(),
                ))),
                origin: None,
            }],
            kept: vec![node],
        }
    };
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation,
        }],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(0),
            subject: node,
            key: vec![],
            headings: vec![],
        },
    }
}

/// Only declared relevant intents change; the installed engine and public entry stay identical.
pub(super) fn measure(total: usize, demanded: &[usize]) -> Sample {
    let engine = engine();
    let prepared = prepared(&engine, total);
    let corpus = corpus();
    let request = engine.begin(&prepared, &corpus);
    let mut intents = vec![FileRuleIntent::Undemanded; total];
    for &index in demanded {
        intents[index] = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        };
    }
    let reads = Cell::new(0);
    let read = |_| {
        reads.set(reads.get() + 1);
        Err(OperatingError::NotRead)
    };
    let elements = Elements::default();
    let observed = Activation::default();
    let dispatch = tracing::Dispatch::new(observed.clone());
    // The first actual opening warms callsite registration and the allocator's
    // own metadata. The second sees exactly the same real request and intents.
    for _ in 0..2 {
        tracing::dispatcher::with_default(&dispatch, || {
            let mut symbols = Symbols::default();
            let file = request
                .open_file(
                    FileInput {
                        language: SourceLanguage::Recognized,
                        resident_stability: facts::ResidentStability::Stable,
                        file: FileId(0),
                        evaluation: &[],
                        publication: &[],
                        intents: &intents,
                        source: b"",
                        elements: &elements,
                        read: &read,
                        semantic: None,
                        kinds: None,
                    },
                    &mut symbols,
                )
                .unwrap();
            assert_eq!(file.readiness_counts().evaluations.0, 0);
            file.finish().unwrap();
        });
    }
    assert_eq!(reads.get(), 0, "activation must not evaluate a provider");
    request.finish();
    let state = observed.0.lock().unwrap();
    assert_eq!((state.starts, state.ends, state.active), (2, 2, false));
    state.sample
}
fn engine() -> ExecutionUseCaseImpl {
    ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    )
}
fn corpus() -> [CorpusInput; 1] {
    [CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("activation.fixture".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
    })]
}
fn prepared(engine: &ExecutionUseCaseImpl, total: usize) -> PreparedProgram {
    engine
        .prepare(PreparedRequest {
            plans: (0..total).map(plan).collect(),
            bindings: vec![vec![]; total],
            rules: (0..total)
                .map(|index| RuleSpec {
                    id: RuleId(format!("activation-{index}").into()),
                    query: QueryBundle {
                        root: QueryDocumentSource {
                            id: QuerySourceId(format!("activation-{index}").into()),
                            text: "".into(),
                        },
                        modules: Default::default(),
                    },
                    applicability: RuleApplicability::Always,
                    given: vec![],
                    observations: vec![],
                    spelled: RuleParameters("".into()),
                    include: vec![],
                    exclude: vec![],
                })
                .collect(),
            digests: (0..total).map(|index| Digest([index as u8; 16])).collect(),
            hosted: vec![],
            observations: ObservationIdentity::Independent,
        })
        .unwrap()
}

/// Compiled frontiers are empty, so no source row or kept bucket may be requested.
struct NoFacts;
impl facts::FactsFinish for NoFacts {
    fn kept(&self, _: Bucket) -> Result<facts::KeptBucket, facts::FactsError> {
        panic!("an activation rule has no file frontier to fetch")
    }
}
impl facts::FactsWorker for NoFacts {
    fn resident_stability(&self) -> facts::ResidentStability {
        facts::ResidentStability::Stable
    }
    fn rows(&self, _: Relation) -> Result<ColumnsView<'_>, facts::FactsError> {
        panic!("an activation rule has no project relation to read")
    }
    fn columns_mut(&mut self) -> Result<facts::ColumnsMut<'_>, facts::FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn keep(&self, _: Bucket, _: facts::WordsToKeep<'_>) -> Result<(), facts::FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn release(&mut self) -> Result<(), facts::FactsError> {
        panic!("an activation rule writes no facts")
    }
    fn finished(self: Box<Self>) -> Result<Box<dyn facts::FactsFinish>, facts::FactsError> {
        panic!("an activation rule writes no facts")
    }
}

/// The public project entry for the declared roots in `demanded` (ascending), observed from
/// phase opening to the last delivery. `intrusion` runs inside the observed window and stands
/// for work no production counter reports; the allocator still sees what it allocates.
pub(super) fn measure_project(
    total: usize,
    demanded: &[usize],
    intrusion: &dyn Fn(usize),
) -> (Sample, Access) {
    let engine = engine();
    let prepared = prepared(&engine, total);
    let corpus = corpus();
    let covered = [Covered {
        file: FileId(0),
        rules: demanded.iter().copied().map(RuleIndex).collect(),
        partial: vec![],
    }];
    let observed = Activation::default();
    let dispatch = tracing::Dispatch::new(observed.clone());
    let project_symbols = Symbols::default();
    // The first cycle warms callsite registration and allocator metadata; the second
    // sees exactly the same real request and declared roots.
    for _ in 0..2 {
        let request = engine.begin(&prepared, &corpus);
        let partials = Partials::default();
        let mut symbols = Symbols::default();
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::trace!(target: "ryo::diagnostic::activation", event = "begin");
            let phase = request.project_phase().unwrap();
            let mut scopes = Vec::with_capacity(demanded.len());
            let mut frontiers = Vec::with_capacity(demanded.len());
            for &rule in demanded {
                let scope = phase
                    .project_scope(RuleIndex(rule), &corpus, &covered, &[])
                    .unwrap();
                frontiers.push(
                    request
                        .prepare_frontiers(
                            FrontierRequest {
                                demand: FrontierDemand::Diagnostic,
                                scope: &scope,
                                rule: RuleIndex(rule),
                                first: Bucket(0),
                                facts: &NoFacts,
                                project: &NoFacts,
                                project_symbols: &project_symbols,
                            },
                            &mut symbols,
                        )
                        .unwrap(),
                );
                scopes.push(scope);
            }
            let tasks = demanded
                .iter()
                .zip(&scopes)
                .zip(frontiers)
                .map(|((&rule, scope), ordinary)| ProjectTask {
                    rule: RuleIndex(rule),
                    demand: RuleDemand {
                        emit: EmitDemand::Enabled,
                        observations: ObservationDemand::NotRequired,
                    },
                    input: ProjectInput {
                        scope,
                        evaluation: &partials,
                        publication: &partials,
                        ordinary,
                        bound: &[],
                        arenas: &[],
                    },
                })
                .collect();
            let ProjectBatch::Complete(outcomes) =
                phase.evaluate(tasks, &mut symbols, |_| Ok::<(), std::convert::Infallible>(()))
            else {
                panic!("an infallible guard cannot refuse")
            };
            assert_eq!(outcomes.len(), demanded.len());
            for outcome in outcomes {
                outcome.unwrap();
            }
            intrusion(total);
            tracing::trace!(target: "ryo::diagnostic::activation", event = "end");
            drop(scopes);
            phase.finish().unwrap();
        });
        request.finish();
    }
    let state = observed.0.lock().unwrap();
    assert_eq!((state.starts, state.ends, state.active), (2, 2, false));
    (state.sample, state.access)
}
