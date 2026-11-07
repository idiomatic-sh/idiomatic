//! Runtime admission fixtures: the omitting adapter and the obligations it is compared against.
pub(super) mod adapter;
pub(super) mod events;
use self::adapter::ReadyAdapter;
use self::events::Events;
use super::super::*;
use crate::application::file_dag::program::kernel_session::KernelSession;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
use std::cell::Cell as Counter;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Obligation {
    pub(crate) generation: String,
    pub(crate) population: String,
    pub(crate) class: u64,
}
pub(crate) struct Coverage {
    pub(crate) expected: BTreeSet<Obligation>,
    pub(crate) served: BTreeSet<Obligation>,
    pub(crate) dispatches: usize,
    pub(crate) skipped: usize,
    pub(crate) heads: usize,
}
pub(crate) fn numbers(value: &str) -> Vec<u64> {
    let value = value.strip_prefix('[').unwrap().strip_suffix(']').unwrap();
    if value.trim().is_empty() {
        return vec![];
    }
    value
        .split(',')
        .map(|number| number.trim().parse().unwrap())
        .collect()
}
pub(crate) fn population(value: &str) -> String {
    value
        .strip_prefix("Ok(")
        .unwrap()
        .strip_suffix(')')
        .unwrap()
        .into()
}
pub(crate) fn decode(events: &Events, skipped: usize, required_heads: BTreeSet<u64>) -> Coverage {
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
pub(crate) fn observe(omit: bool) -> Coverage {
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
                        vec![PlanComparison {
                            test: Test::Equal,
                            left: Operand::Literal(QueryLiteral::Number(Integer(index))),
                            right: Operand::Literal(QueryLiteral::Number(Integer(index))),
                            origin:
                                ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
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
            ObservationIdentity::Independent, crate::domain::executable_program::types::RoutePolicy::default(),
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
pub(crate) fn registration_probe(omission: bool) {
    if omission {
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = "capture_registration_omission");
    } else {
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = "capture_registration_actual");
    }
}
