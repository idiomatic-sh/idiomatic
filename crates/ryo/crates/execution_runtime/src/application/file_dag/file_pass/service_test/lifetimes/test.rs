//! How long a node's rows live in a file, through the request's per-file route, seen at the operators port: a node's rows go
//! once its last consumer is done, whichever comes last of its node readers and its engine uses (the rules that emit it, the
//! frontiers and named outputs that hold it); never before, so no node is made twice in a file; and never past the file's end.
use super::super::fixtures::{RUN, every, every_node, first_column, node_of, plan, ran, request};
use ryo_execution_domain::domain::rule_intent::types::{ApplicabilityReason, FileRuleIntent};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, StepId, StepScope};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use std::borrow::Cow;
use std::sync::PoisonError;

/// The request and the file a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Rule 0 emits the functions and hands them over; no node reads them; rule 1 emits the comments.
    LastEngineUse,
    /// Rule 0 emits the functions; rule 1 projects the same functions' first column.
    EngineUseThenReader,
    /// Rule 0 projects the functions' first column and hands the functions over.
    ReadThenCaptured,
    /// Rule 0 and rule 2 emit the functions; rule 1's emitted comments fault, which stops the file before rule 2.
    StoppedFile,
    /// Rule 0 keeps the functions of no comment's file, a probe of the comments keyed by file; rule 1, which does not run on
    /// the file, and rule 2 emit one node, a second such probe. Rule 1's turn ends before any rule made that node: the later
    /// rule makes it once and holds it. (How many indexes the probes build is counted at seam C, in operators' shared_index
    /// cases: an_engine_use_of_a_last_probe_not_yet_made_keeps_its_shared_index.)
    SkippedSharesALastProbe,
}

type Input = Scenario;
/// What the operators saw, in order: each run with whether each watched node held rows before and after it, and each relation
/// lent; for a stopped file, what the next file's first run saw.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_buffer_is_released_after_its_last_consumer"),
        word: Word::Succeeds,
        input: || Scenario::LastEngineUse,
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 0 before 0:held; ran 0 after 0:held; run 1 before 0:absent; lend Comment; ran 1 after 0:absent",
        )),
    },
    Case {
        name: CaseName("an_engine_use_before_a_later_node_reader_keeps_the_rows"),
        word: Word::Succeeds,
        input: || Scenario::EngineUseThenReader,
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 1 before 0:held; ran 1 after 0:absent",
        )),
    },
    Case {
        name: CaseName("a_node_read_then_captured_is_made_once"),
        word: Word::Succeeds,
        input: || Scenario::ReadThenCaptured,
        expect: Expect::Value(Cow::Borrowed(
            "run 1 before 0:absent; lend Function; ran 1 after 0:held; run 0 before 0:held; ran 0 after 0:held",
        )),
    },
    Case {
        name: CaseName("a_rule_not_run_that_shares_a_node_leaves_it_to_the_later_rule"),
        word: Word::Succeeds,
        input: || Scenario::SkippedSharesALastProbe,
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent 2:absent; lend Function; lend Comment; ran 0 after 0:held 2:absent; run 2 before 0:absent 2:absent; lend Function; lend Comment; ran 2 after 0:absent 2:held",
        )),
    },
    Case {
        name: CaseName("no_buffer_outlives_the_file"),
        word: Word::Succeeds,
        input: || Scenario::StoppedFile,
        expect: Expect::Value(Cow::Borrowed(
            "rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 1: failed WrongOperation; unreached. unfinished; partial [] next: run 0 before 0:absent 1:absent",
        )),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let functions = || plan(vec![every(Relation::Function)], 0, Vec::new());
    let comments = || plan(vec![every(Relation::Comment)], 0, Vec::new());
    let handed = vec![StepInput::Step(StepId(0))];
    let plans = match input {
        Scenario::LastEngineUse => {
            vec![plan(vec![every(Relation::Function)], 0, handed), comments()]
        }
        Scenario::EngineUseThenReader => vec![
            functions(),
            plan(
                vec![every(Relation::Function), first_column(0)],
                1,
                Vec::new(),
            ),
        ],
        Scenario::ReadThenCaptured => vec![plan(
            vec![every(Relation::Function), first_column(0)],
            1,
            handed,
        )],
        Scenario::StoppedFile => vec![functions(), comments(), functions()],
        Scenario::SkippedSharesALastProbe => {
            let unremarked = |left: StepInput| PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Antijoin {
                    left,
                    right: StepInput::Relation(Relation::Comment),
                    left_on: vec![ColumnIndex(2)],
                    right_on: vec![ColumnIndex(2)],
                },
            };
            let later = || {
                plan(
                    vec![
                        every(Relation::Function),
                        unremarked(StepInput::Step(StepId(0))),
                    ],
                    1,
                    Vec::new(),
                )
            };
            vec![
                plan(
                    vec![unremarked(StepInput::Relation(Relation::Function))],
                    0,
                    Vec::new(),
                ),
                later(),
                later(),
            ]
        }
    };
    let mut intents = vec![RUN; plans.len()];
    if let Scenario::SkippedSharesALastProbe = input
        && let Some(skipped) = intents.get_mut(1)
    {
        *skipped = FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable);
    }
    let (request, watch) = request(plans);
    let functions = node_of(&request, 0, 0);
    if let Scenario::StoppedFile = input {
        *watch.failing.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(node_of(&request, 1, 0));
        let stopped = ran(&request, &intents, &[]);
        *watch.failing.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let _stopped_file = watch.taken();
        watch.watch(&every_node(&request));
        let _next = ran(&request, &intents, &[]);
        let first = watch.taken().into_iter().next();
        return Ok(Cow::Owned(format!(
            "{} next: {}",
            stopped.rules,
            first.unwrap_or_default()
        )));
    }
    if let Scenario::SkippedSharesALastProbe = input {
        // Both probes watched: the first, and the second, the shared index's last probe, which the skipped rule emits.
        watch.watch(&[functions, node_of(&request, 1, 1)]);
    } else {
        watch.watch(&[functions]);
    }
    let _ran = ran(&request, &intents, &[]);
    Ok(Cow::Owned(watch.taken().join("; ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
