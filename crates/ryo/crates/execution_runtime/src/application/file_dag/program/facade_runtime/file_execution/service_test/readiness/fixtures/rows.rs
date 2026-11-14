//! The readiness cases: their rows, the observation they read and the subject that runs them.
use super::*;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::facade_runtime::source_admission::service::SourceInputs;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
#[cfg(test)]
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use std::borrow::Cow;
pub(crate) fn observe(omit: bool) -> Observed {
    let events = Events::default();
    let observer = tracing::Dispatch::new(events.clone());
    // tracing-core 0.1.36's sole-dispatch fast path consults the registering
    // thread's default. Another libtest thread may have no local observer.
    // Two live registrations force callsite interest to consult the registered
    // subscribers instead. Only `observer` is selected for event delivery.
    let registration = tracing::Dispatch::new(events.clone());
    let mut skipped = 0;
    let mut unannounced = 0;
    let mut required_heads = BTreeSet::new();
    let mut probe_events = (0, 0);
    let mut roots = Vec::new();
    let mut runs = Vec::new();
    let mut replay_events = 0;
    tracing::dispatcher::with_default(&observer, || {
        let before = events.0.lock().unwrap().len();
        std::thread::spawn(move || registration_probe(omit))
            .join()
            .unwrap();
        // The unrelated thread has no capture dispatcher; a callsite first registered without a
        // local subscriber remains visible to this capture.
        let unrelated = events.0.lock().unwrap().len() - before;
        registration_probe(omit);
        probe_events = (
            unrelated,
            events.0.lock().unwrap().len() - before - unrelated,
        );
        let SmallTree {
            elements,
            mut symbols,
        } = small_tree();
        let mut plans = Vec::new();
        let mut rules = Vec::new();
        for index in 0..5 {
            let mut authored = plan(PlanShape::ProjectRetry);
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
        let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
        let prepared = crate::application::file_dag::program::preparation::service::prepare(
            Compiled {
                schedule: RuleSchedule::Independent,
                observations: vec![vec![]; 5],
                plans: plans.into_iter().map(std::sync::Arc::new).collect(),
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
            let head = &region.instructions()[0];
            roots.push(Root {
                phase: region.phase(),
                instructions: region.instructions().len(),
                reads_a_relation: matches!(
                    &head.operation(),
                    Operation::Select {
                        from: PlanInput::Relation(_),
                        ..
                    } | Operation::Project {
                        from: PlanInput::Relation(_),
                        distinct: RepeatedRows::Kept,
                        ..
                    }
                ),
                output_kinds: head.contract().output_kinds.as_ref().ok().map(Vec::len),
                new_head: required_heads.insert(u64::try_from(head.logical().0).unwrap()),
            });
        }
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(Arc::new(
                crate::application::file_dag::fixtures::operators(),
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
            ReuseEligibility::Eligible,
            &mut symbols,
            None,
            None,
        )
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
            Some(prepared.program().regions()[root.0].instructions()[0].logical())
        } else {
            None
        };
        {
            let ScopePartials::File {
                evaluation,
                publication,
            } = &file.partials
            else {
                panic!("the fixture opens a file scope")
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
                unannounced: Counter::new(0),
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
            unannounced = adapter.unannounced.get();
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
                if let Some(expected) = first.as_ref() {
                    runs.push((table.count.0, &table.cells == expected));
                } else {
                    runs.push((table.count.0, true));
                    first = Some(table.cells);
                }
            }
        }
        // Retained publication and equivalent-head replay add no readiness or dispatch.
        replay_events = events.0.lock().unwrap().len() - before;
    });
    drop(registration);
    let coverage = decode(&events, skipped, required_heads);
    Observed {
        checks: Checks {
            probe_events,
            roots: roots.into(),
            runs: runs.into(),
            replay_events,
            unannounced,
            events: coverage.events,
        },
        coverage,
    }
}

/// One authored emit root's region: its phase, its instruction count, whether its head reads a
/// relation (a select, or a project keeping repeats), its head's output kinds, and whether its
/// head was new among the roots.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in super::super) struct Root {
    pub(in super::super) phase: StepScope,
    pub(in super::super) instructions: usize,
    pub(in super::super) reads_a_relation: bool,
    pub(in super::super) output_kinds: Option<usize>,
    pub(in super::super) new_head: bool,
}

/// What the run showed beside its obligations: the events the unrelated thread's and then the
/// local registration probe added, each authored root's region, each rule's two runs (the rows
/// answered, and whether the run answered the first run's cells), the events the retained
/// publication and replay added, the ready callbacks with no obligation before them, and what
/// the decoded events show.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Checks {
    pub(in super::super) probe_events: (usize, usize),
    pub(in super::super) roots: Cow<'static, [Root]>,
    pub(in super::super) runs: Cow<'static, [(usize, bool)]>,
    pub(in super::super) replay_events: usize,
    pub(in super::super) unannounced: usize,
    pub(in super::super) events: EventChecks,
}

/// The decoded coverage and the checks beside it.
pub(crate) struct Observed {
    pub(crate) coverage: Coverage,
    pub(crate) checks: Checks,
}

#[derive(Debug, PartialEq)]
pub(in super::super) enum Refusal {
    MissingSubmission,
}

/// What runtime admission served: the dispatches, the distinct obligations, the ready heads,
/// and the submissions the adapter omitted.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Admitted {
    pub(in super::super) dispatches: usize,
    pub(in super::super) obligations: usize,
    pub(in super::super) heads: usize,
    pub(in super::super) skipped: usize,
    pub(in super::super) checks: Checks,
}

/// The refusal, what was admitted, and how many obligations were never served.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Unserved {
    pub(in super::super) refusal: Refusal,
    pub(in super::super) admitted: Admitted,
    pub(in super::super) unserved: usize,
}

/// Admits the corpus through the ready adapter and compares the obligations with those served.
pub(in super::super) fn exercise(submission: Submission) -> Result<Admitted, Unserved> {
    let Observed {
        coverage: observed,
        checks,
    } = observe(matches!(submission, Submission::Omitted));
    let admitted = Admitted {
        dispatches: observed.dispatches,
        obligations: observed.expected.len(),
        heads: observed.heads,
        skipped: observed.skipped,
        checks,
    };
    let unserved = observed.expected.difference(&observed.served).count();
    if observed.expected == observed.served {
        Ok(admitted)
    } else {
        Err(Unserved {
            refusal: Refusal::MissingSubmission,
            admitted,
            unserved,
        })
    }
}
