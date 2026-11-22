//! The programs and runs of the `refusal_order` cases: a hosted text, steps over it, and how a run of them shows.
pub(super) mod programs;
use crate::*;
use ryo_execution::inbound::use_cases::execution::interface::LogicalCounts;
use ryo_execution::inbound::use_cases::execution::interface::{
    Arena, CaptureContext, FileRules, Partials, PhasePartials, PhaseRules, ProgramProjectPhasePort,
    ProjectBatch, ProjectRootExecution,
};
use ryo_execution::inbound::use_cases::execution::interface::{ObserverPort, Observing};
use ryo_execution_runtime::outbound::facts::fixtures::project_phase_axis::CapturedBuckets;
use ryo_execution_runtime::outbound::observer::types::FedTotals;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::row_texts::types::Cell as HostCell;
use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket};
use std::sync::Mutex;

/// An observer that keeps what each rule was told it was fed, in order.
#[derive(Default)]
struct Told(Mutex<Vec<String>>);

impl ObserverPort for Told {
    fn fed(&self, rule: RuleIndex, counts: LogicalCounts) -> Result<(), OperatingError> {
        self.0
            .lock()
            .expect("an unpoisoned record")
            .push(format!("r{}{:?}", rule.0, counts.fed));
        Ok(())
    }
    fn totals(&self) -> Result<FedTotals, OperatingError> {
        Ok(FedTotals(Vec::new()))
    }
    fn handed(
        &self,
        _: ryo_values::domain::runs::types::FileHandover,
    ) -> Result<(), OperatingError> {
        Ok(())
    }
    fn handovers(
        &self,
    ) -> Result<ryo_execution_runtime::outbound::observer::types::FileHandovers, OperatingError>
    {
        Ok(ryo_execution_runtime::outbound::observer::types::FileHandovers::default())
    }
}

/// The observer of a run, and what it was told, when the run is observed.
fn observing(observe: bool) -> (Option<Arc<Told>>, Observing<dyn ObserverPort>) {
    if observe {
        let told = Arc::new(Told::default());
        (Some(Arc::clone(&told)), Observing::Observer(told))
    } else {
        (None, Observing::Unobserved)
    }
}

/// What the observer was told, as the cases show it; nothing for an unobserved run.
fn told(observer: Option<&Arc<Told>>) -> String {
    observer.map_or(String::new(), |told| {
        format!(" fed={:?}", told.0.lock().expect("an unpoisoned record"))
    })
}

/// The hosted `text(value: Text, n: Number)`, with the row `["x", 1]` or none.
fn text(populated: bool) -> HostRelation {
    HostRelation {
        name: HostName("text".into()),
        columns: vec![
            HostColumn {
                name: HostColumnName("value".into()),
                kind: HostKind::Text,
            },
            HostColumn {
                name: HostColumnName("n".into()),
                kind: HostKind::Number,
            },
        ],
        rows: if populated {
            vec![HostRow(vec![
                HostCell::Text("x".into()),
                HostCell::Number(Integer(1)),
            ])]
        } else {
            vec![]
        },
    }
}

/// A root's answer, as the cases show it.
fn shown(outcome: Result<RootOutcome, FileStop>) -> String {
    match outcome {
        Ok(RootOutcome::Complete(table)) => format!("complete {:?} {:?}", table.kinds, table.count),
        Ok(RootOutcome::Partial { table, .. }) => format!("partial {:?}", table.kinds),
        Ok(RootOutcome::Failed(error)) => format!("failed {error:?}"),
        Ok(RootOutcome::Unknown(error)) => format!("unknown {error:?}"),
        Ok(RootOutcome::Deferred { .. }) => "deferred".into(),
        Ok(RootOutcome::Skipped(_) | RootOutcome::NoQuery) => "other".into(),
        Err(_) => "filestop".into(),
    }
}

/// The one-rule request of `steps` emitting `emit`, with `frontier`, over the hosted text.
fn request(
    steps: &[(StepScope, Operation)],
    emit: u32,
    frontier: Vec<Input>,
    populated: bool,
) -> PrepareRequest {
    let mut authored = authored_request(1);
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    authored.hosted = vec![text(populated)];
    let plan = &mut boundary_fixture::fixtures::plans(&mut authored)[0];
    plan.steps = steps
        .iter()
        .map(|(scope, operation)| PlanStep {
            written: Default::default(),
            scope: *scope,
            operation: operation.clone(),
        })
        .collect();
    plan.emit.step = StepId(emit);
    plan.frontier = frontier;
    authored
}

/// The intent every case runs its rule with.
const COLLECTED: FileRuleIntent = FileRuleIntent::Run {
    diagnostic: DiagnosticDelivery::Collected,
    observations: ObservationDemand::NotRequired,
    capture: CaptureIntent::DiagnosticAndObservations,
};

/// What rule 0 of `steps` answers in file 0, collected, over `facts`, and what the observer was told.
pub(super) fn per_file(
    steps: &[(StepScope, Operation)],
    emit: u32,
    populated: bool,
    observe: bool,
    facts: &[(Relation, &Table)],
) -> String {
    let prepared = engine()
        .prepare(request(steps, emit, vec![], populated))
        .expect("prepare the rule");
    let corpus = corpus();
    let (observer, observing) = observing(observe);
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, observing);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
    let mut facts = TableFacts::new(facts);
    let rules = run_file_into(
        &request,
        FileRun::of(&[COLLECTED], &elements),
        &mut symbols,
        CaptureContext {
            worker: WorkerNumber(0),
            arena: Arena(&mut arena),
            places: &mut places,
            words: &mut words,
        },
        &mut facts,
    );
    let answer = file_answer(rules);
    request.finish();
    format!("{answer}{}", told(observer.as_ref()))
}

/// Rule 0's answer in a file run, or how the file ended when it never ran.
fn file_answer(mut rules: FileRules) -> String {
    match rules.rules.iter().position(|each| each.rule.0 == 0) {
        Some(at) => shown(root(rules.rules.remove(at))),
        None => match rules.end {
            FileEnd::Refused(stop) => format!("refused {stop:?}"),
            FileEnd::Finished(_) | FileEnd::Unfinished => "not run".into(),
        },
    }
}

/// What the project phase answers for rule 0 of `steps`, whose only file was skipped, and what the observer was told.
pub(super) fn after_the_files(
    steps: &[(StepScope, Operation)],
    emit: u32,
    frontier: Vec<Input>,
    populated: bool,
    observe: bool,
) -> String {
    let mut authored = request(steps, emit, frontier, populated);
    let skipped = vec![Skipped {
        rule: authored.rules[0].id.clone(),
        path: FilePath("input.py".into()),
        reason: SkipReason::SyntaxRejected,
    }];
    authored.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let prepared = engine().prepare(authored).expect("prepare the rule");
    let corpus = corpus();
    let (observer, observing) = observing(observe);
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, observing);
    let none = CapturedBuckets {
        captures: Vec::new(),
    };
    let answer = phase_answer(&request, &corpus, &skipped, &[], &none, &[]);
    request.finish();
    format!("{answer}{}", told(observer.as_ref()))
}

/// What the project phase of `request` answers for rule 0, its files covered by `covered` and skipped by `skipped`.
fn phase_answer(
    request: &ryo_execution::inbound::use_cases::execution::interface::RequestExecution,
    corpus: &[ryo_values::domain::files::types::CorpusInput],
    skipped: &[Skipped],
    covered: &[ryo_execution::inbound::use_cases::execution::interface::Covered],
    facts: &CapturedBuckets,
    arenas: &[ArenaBytes],
) -> String {
    let project_symbols = Symbols::default();
    // No relation of the project is read: every program reads the hosted text.
    let project = TableFacts::new(&[]);
    let inapplicable =
        vec![ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default()];
    let refused =
        vec![ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default()];
    let phase = request.project_phase().expect("the project phase");
    let mut symbols = Symbols::default();
    let batch = phase.evaluate_rules(
        PhaseRules {
            rules: &[RuleIndex(0)],
            first: &[Bucket(0)],
            corpus,
            covered,
            skipped,
            inapplicable: &inapplicable,
            refused: &refused,
            facts,
            project: &project,
            project_symbols: &project_symbols,
            arenas,
        },
        &mut symbols,
        |_| Ok::<(), &str>(()),
        |_| {
            Ok(PhasePartials {
                evaluation: Partials::default(),
                publication: Partials::default(),
            })
        },
        |_| Ok(()),
        |_, outcome, _, _, _| Ok(outcome.emit),
    );
    let mut answer = match batch {
        ProjectBatch::Complete(done) => done
            .into_iter()
            .map(|each| match each {
                Ok(outcome) => shown(Ok(outcome)),
                Err(stop) => format!("stopped {stop:?}"),
            })
            .collect::<Vec<_>>()
            .join(","),
        ProjectBatch::Refused { error, .. } => format!("refused {error:?}"),
    };
    if let Err(error) = phase.finish() {
        answer.push_str(&format!(" finish={error:?}"));
    }
    answer
}

/// Rule 0 of `steps`, some in each file and some after them, run in file 0 and then after the files over what the file
/// captured: the file's answer and what it captured, then the phase's answer, and what the observer was told.
pub(super) fn both_passes(
    steps: &[(StepScope, Operation)],
    emit: u32,
    frontier: Vec<Input>,
    populated: bool,
    observe: bool,
) -> String {
    let prepared = engine()
        .prepare(request(steps, emit, frontier, populated))
        .expect("prepare the rule");
    let corpus = corpus();
    let (observer, observing) = observing(observe);
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, observing);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let (mut arena, mut places, mut words) = (Vec::new(), Vec::new(), Words::default());
    let mut facts = TableFacts::new(&[]);
    let mut rules = run_file_into(
        &request,
        FileRun::of(&[COLLECTED], &elements),
        &mut symbols,
        CaptureContext {
            worker: WorkerNumber(0),
            arena: Arena(&mut arena),
            places: &mut places,
            words: &mut words,
        },
        &mut facts,
    );
    let mut captures = Vec::new();
    let mut captured = Vec::new();
    let file = match rules.rules.iter().position(|each| each.rule.0 == 0) {
        Some(at) => {
            let ran = rules.rules.remove(at);
            let answer = match (ran.emit, ran.after) {
                (EmittedRoot::Delivered(outcome), RuleAfter::Ran { captures: kept }) => {
                    for capture in kept {
                        match capture.outcome {
                            Ok(frontier) => {
                                captured.push("kept".to_owned());
                                captures.push((FileId(0), frontier));
                            }
                            Err(error) => captured.push(format!("refused {error:?}")),
                        }
                    }
                    Ok(outcome)
                }
                (EmittedRoot::Delivered(outcome), RuleAfter::Unreached | RuleAfter::Stopped(_)) => {
                    Ok(outcome)
                }
                (EmittedRoot::Withheld, RuleAfter::Stopped(stop)) => Err(stop),
                (EmittedRoot::Withheld, RuleAfter::Ran { .. } | RuleAfter::Unreached) => {
                    Err(FileStop::Operating(OperatingError::NotRun))
                }
            };
            shown(answer)
        }
        None => "not run".into(),
    };
    let covered = vec![
        ryo_execution::inbound::use_cases::execution::interface::Covered {
            file: FileId(0),
            rules: vec![RuleIndex(0)],
            partial: vec![],
        },
    ];
    let phase = phase_answer(
        &request,
        &corpus,
        &[],
        &covered,
        &CapturedBuckets { captures },
        &[ArenaBytes(arena)],
    );
    request.finish();
    format!(
        "file={file} captured={captured:?} phase={phase}{}",
        told(observer.as_ref())
    )
}
