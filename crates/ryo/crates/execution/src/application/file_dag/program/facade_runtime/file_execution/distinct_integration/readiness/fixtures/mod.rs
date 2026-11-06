//! The negative adapter omits only ready; all evaluation stays in KernelSession.
use super::super::*;
use crate::application::file_dag::program::{
    kernel_session::KernelSession,
    runtime::{
        service::{KernelInvocation, KernelPermit, ReadySelection},
        types::{ColumnCounters, InvocationFailure, KernelInput, KernelScratch, StrategyOutcome},
    },
};
use crate::domain::{
    executable_program::types::{Instruction, LogicalInstruction},
    operating::types::Emit as RowSink,
};
use ryo_values::domain::plan_parts::types::{Comparison, Operand, QueryLiteral, Test};
use std::{
    cell::Cell as Counter,
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<BTreeMap<String, String>>>>);
#[derive(Default)]
struct Fields(BTreeMap<String, String>);
impl tracing::field::Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let _previous = self.0.insert(field.name().into(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let _previous = self.0.insert(field.name().into(), value.into());
    }
}
impl tracing::Subscriber for Events {
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
        metadata.target() == crate::application::file_dag::program::constants::WALK_TARGET
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.0.lock().unwrap().push(fields.0);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}
struct ReadyAdapter<'a> {
    actual: &'a dyn KernelInvocation,
    omitted: Option<LogicalInstruction>,
    skipped: Counter<usize>,
    events: Events,
}
impl KernelInvocation for ReadyAdapter<'_> {
    fn ready(
        &self,
        ready: &ReadySelection<'_, '_>,
        inputs: &[AdmittedInput],
        context: &mut OperatorContext<'_>,
    ) -> Result<(), OperatingError> {
        let logical = ready.permit().descriptor().logical;
        {
            let events = self.events.0.lock().unwrap();
            assert!(
                events
                    .iter()
                    .any(|event| event.get("event").map(String::as_str)
                        == Some(crate::application::file_dag::program::constants::WALK_READY)
                        && event.get("logical") == Some(&logical.0.to_string())),
                "actual runtime obligation must exist before this callback, even when it will be skipped; head={logical:?}; operation={:?}; captured={events:?}",
                ready.permit().descriptor().operation
            );
        }
        if self.omitted == Some(logical) {
            self.skipped.set(self.skipped.get() + 1);
            return Ok(());
        }
        self.actual.ready(ready, inputs, context)
    }
    fn ordered(
        &self,
        shape: &crate::domain::executable_program::types::OrderedExistence,
        permit: &KernelPermit<'_>,
        input: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn RowSink,
    ) -> Result<StrategyOutcome, InvocationFailure> {
        self.actual
            .ordered(shape, permit, input, context, scratch, output)
    }
    fn positions(
        &self,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<crate::application::file_dag::relations::handle::service::PublishedInput>,
        InvocationFailure,
    > {
        self.actual.positions(permit, context, scratch)
    }
    fn selected(
        &self,
        shape: &crate::domain::executable_program::selection::SelectionAlternative,
        permit: &KernelPermit<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
    ) -> Result<
        crate::application::file_dag::program::runtime::types::Attempt<crate::application::file_dag::relations::handle::service::PublishedInput>,
        InvocationFailure,
    > {
        self.actual.selected(shape, permit, context, scratch)
    }
    fn column(
        &self,
        shape: &crate::domain::executable_program::column::ColumnAlternative,
        permit: &KernelPermit<'_>,
        left: &AdmittedInput,
        right: &AdmittedInput,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        counts: &ColumnCounters,
        rows: &mut Table,
    ) -> Result<StrategyOutcome, InvocationFailure> {
        self.actual
            .column(shape, permit, left, right, context, scratch, counts, rows)
    }
    fn evaluate(
        &self,
        instruction: &Instruction,
        permit: &KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn RowSink,
    ) -> Result<(), InvocationFailure> {
        self.actual
            .evaluate(instruction, permit, input, context, scratch, output)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Obligation {
    generation: String,
    population: String,
    class: u64,
}
pub(super) struct Coverage {
    pub(super) expected: BTreeSet<Obligation>,
    pub(super) served: BTreeSet<Obligation>,
    pub(super) dispatches: usize,
    pub(super) skipped: usize,
    pub(super) heads: usize,
}
fn numbers(value: &str) -> Vec<u64> {
    let value = value.strip_prefix('[').unwrap().strip_suffix(']').unwrap();
    if value.trim().is_empty() {
        return vec![];
    }
    value
        .split(',')
        .map(|number| number.trim().parse().unwrap())
        .collect()
}
fn population(value: &str) -> String {
    value
        .strip_prefix("Ok(")
        .unwrap()
        .strip_suffix(')')
        .unwrap()
        .into()
}
fn decode(events: &Events, skipped: usize, required_heads: BTreeSet<u64>) -> Coverage {
    let mut heads = BTreeMap::new();
    let mut expected = BTreeSet::new();
    let mut served = BTreeSet::new();
    let mut dispatches = 0;
    let mut ready_heads = 0;
    let mut actual_heads = BTreeSet::new();
    let events = events.0.lock().unwrap();
    for event in events.iter().filter(|event| {
        event.get("event").map(String::as_str)
            == Some(crate::application::file_dag::program::constants::WALK_CLASS)
    }) {
        for head in numbers(&event["heads"]) {
            assert!(
                heads
                    .insert(head, event["class"].parse::<u64>().unwrap())
                    .is_none()
            );
        }
    }
    for event in events.iter() {
        match event.get("event").map(String::as_str) {
            Some(crate::application::file_dag::program::constants::WALK_READY) => {
                assert_eq!(event["phase"], "PerFile");
                assert_eq!(event["file"], "0");
                let head = event["logical"].parse::<u64>().unwrap();
                assert!(
                    actual_heads.insert(head),
                    "one authority-issued readiness obligation per physical head in this valid demand-all corpus"
                );
                expected.insert(Obligation {
                    generation: event["generation"].clone(),
                    population: population(&event["population"]),
                    class: *heads
                        .get(&head)
                        .expect("missing actual compiled physical-head mapping"),
                });
                ready_heads += 1;
            }
            Some(crate::application::file_dag::program::constants::WALK_DISPATCH) => {
                dispatches += 1;
                for class in numbers(&event["classes"]) {
                    assert!(
                        served.insert(Obligation {
                            generation: event["generation"].clone(),
                            population: event["population"].clone(),
                            class
                        }),
                        "same class/population/generation dispatched twice"
                    );
                }
            }
            Some(crate::application::file_dag::program::constants::WALK_REFUSED) => {
                panic!("this valid typed corpus has no representation refusal: {event:?}")
            }
            _ => {}
        }
    }
    assert_eq!(
        actual_heads, required_heads,
        "all independently declared and compiled roots of this demand-all corpus must reach runtime admission"
    );
    Coverage {
        expected,
        served,
        dispatches,
        skipped,
        heads: ready_heads,
    }
}

pub(super) fn observe(omit: bool) -> Coverage {
    let events = Events::default();
    let observer = tracing::Dispatch::new(events.clone());
    // tracing-core 0.1.36's sole-dispatch fast path consults the registering
    // thread's default. Another libtest thread may have no local observer.
    // Two live registrations force callsite interest to consult the registered
    // subscribers instead. Only `observer` is selected for event delivery.
    let registration = tracing::Dispatch::new(events.clone());
    let mut skipped = 0;
    let mut required_heads = BTreeSet::new();
    tracing::dispatcher::with_default(&observer, || {
        let before = events.0.lock().unwrap().len();
        std::thread::spawn(move || registration_probe(omit))
            .join()
            .unwrap();
        assert_eq!(
            events.0.lock().unwrap().len(),
            before,
            "the unrelated thread has no capture dispatcher"
        );
        registration_probe(omit);
        assert_eq!(
            events.0.lock().unwrap().len(),
            before + 1,
            "a callsite first registered without a local subscriber remains visible to this capture"
        );
        let SmallTree {
            elements,
            mut symbols,
        } = small_tree();
        let mut plans = Vec::new();
        let mut rules = Vec::new();
        for index in 0..5 {
            let mut authored = plan(Scenario::ProjectRetry);
            authored.steps.truncate(1);
            authored.emit.step = StepId(0);
            let relation = if index == 3 {
                Relation::SyntaxTree
            } else {
                Relation::Function
            };
            authored.steps[0].operation = if index == 4 {
                Operation::Project {
                    from: PlanInput::Relation(relation),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                }
            } else {
                Operation::Select {
                    from: PlanInput::Relation(relation),
                    kept: vec![ColumnIndex(0)],
                    comparisons: if index == 1 || index == 2 {
                        vec![Comparison {
                            test: Test::Equal,
                            left: Operand::Literal(QueryLiteral::Number(Integer(index))),
                            right: Operand::Literal(QueryLiteral::Number(Integer(index))),
                            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                        }]
                    } else {
                        vec![]
                    },
                }
            };
            plans.push(authored);
            let mut spec = rule();
            spec.id = RuleId(format!("ready-{index}").into());
            rules.push(spec);
        }
        let derived = crate::application::file_dag::testing::program_derived_port();
        let prepared = crate::application::file_dag::program::preparation::service::prepare(
            Compiled {
                schedule: RuleSchedule::Independent,
                observations: vec![vec![]; 5],
                plans,
                bindings: vec![vec![]; 5],
            },
            &rules,
            &[Digest([0; 16]); 5],
            &[],
            Arc::clone(&derived),
            ObservationIdentity::Independent,
        )
        .unwrap();
        // Expected physical IDs come from all five authored emit roots, not
        // prepared walk classes, ready observations or executed dispatches.
        for rule in 0..rules.len() {
            let root = prepared
                .rule_metadata(RuleIndex(rule))
                .unwrap()
                .emit_root
                .clone()
                .unwrap();
            let region = &prepared.program().regions()[root.0];
            assert_eq!(region.phase, StepScope::PerFile);
            assert_eq!(region.instructions.len(), 1);
            let head = &region.instructions[0];
            assert!(matches!(
                &head.operation,
                Operation::Select {
                    from: PlanInput::Relation(_),
                    ..
                } | Operation::Project {
                    from: PlanInput::Relation(_),
                    distinct: RepeatedRows::Kept,
                    ..
                }
            ));
            assert_eq!(head.contract.output_kinds.as_ref().unwrap().len(), 1);
            assert!(required_heads.insert(u64::try_from(head.logical.0).unwrap()));
        }
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            ))),
            derived,
            &[CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: FilePath("ready.py".into()),
                size: FileSize(SMALL_SOURCE.len() as u64),
                modified: Nanoseconds(0),
            })],
        ));
        let intent = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        };
        let mut file = open_file(
            &prepared,
            FileId(0),
            ScopePartials::File {
                evaluation: &[],
                publication: &[],
            },
            &[intent; 5],
            services,
            ryo_values::domain::files::types::SourceView(SMALL_SOURCE),
            crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
            &mut symbols,
            None,
            None)
        .unwrap();
        let function = function_rows(&[0, 2]);
        let read = |relation| match relation {
            Relation::Function => Ok(Rows::Computed(&function)),
            Relation::SyntaxTree => Ok(Rows::Tree(TreeRows {
                elements: &elements,
                file: FileId(0),
                form: TreeForm::Elements,
            })),
            _ => Err(OperatingError::NotRead),
        };
        let omitted = if omit {
            let root = prepared
                .rule_metadata(RuleIndex(1))
                .unwrap()
                .emit_root
                .clone()
                .unwrap();
            Some(prepared.program().regions()[root.0].instructions[0].logical)
        } else {
            None
        };
        {
            let ScopePartials::File {
                evaluation,
                publication,
            } = &file.partials
            else {
                unreachable!()
            };
            let completion =
                CompletionPolicy::file_inputs(RuleIndex(0), file.file, evaluation, publication);
            let mut context = file
                .context
                .borrow(&mut symbols, file.program, RuleIndex(0))
                .unwrap();
            let actual = KernelSession::file(
                file.services.kernels(),
                file.program,
                Rc::clone(&file.owner),
                &read,
                &file.walk,
            );
            let adapter = ReadyAdapter {
                actual: &actual,
                omitted,
                skipped: Counter::new(0),
                events: events.clone(),
            };
            let mut sources = SourceInputs {
                execution: &file.execution,
                state: file.execution.file_state(),
                completion: completion.clone(),
                owner: Rc::clone(&file.owner),
                read: &read,
                observations: &mut file.observed,
                file_outputs: Some(&file.roots),
                program: file.program,
                rule: RuleIndex(0),
                project: None,
            };
            file.execution
                .prepare_roots(
                    file.execution.file_state(),
                    &file.prepared_roots,
                    &mut sources,
                    &adapter,
                    &completion,
                    &mut context,
                )
                .unwrap();
            skipped = adapter.skipped.get();
        }
        let before = events.0.lock().unwrap().len();
        for index in 0..5 {
            let mut first = None;
            for _ in 0..2 {
                let result = file
                    .rule(
                        RuleIndex(index),
                        RuleDemand {
                            emit: EmitDemand::Enabled,
                            observations: ObservationDemand::NotRequired,
                        },
                        &read,
                        &mut symbols,
                    )
                    .unwrap();
                let RootOutcome::Complete(table) = result.emit else {
                    panic!("actual delegated runtime must complete this valid corpus");
                };
                assert!(table.count.0 > 0);
                if let Some(expected) = first.as_ref() {
                    assert_eq!(&table.cells, expected);
                } else {
                    first = Some(table.cells);
                }
            }
        }
        assert_eq!(
            events.0.lock().unwrap().len(),
            before,
            "retained publication and equivalent-head replay adds no readiness or dispatch"
        );
    });
    drop(registration);
    decode(&events, skipped, required_heads)
}

// Distinct callsites make both CASES exercise first registration independently.
fn registration_probe(omission: bool) {
    if omission {
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = "capture_registration_omission");
    } else {
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = "capture_registration_actual");
    }
}
