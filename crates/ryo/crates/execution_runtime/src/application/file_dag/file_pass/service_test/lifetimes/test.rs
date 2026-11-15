//! How long a node's rows live in a file, through the request's per-file route, seen at the operators port: a node's rows go
//! once its last consumer is done, whichever comes last of its node readers and its engine uses (the rules that emit it, the
//! frontiers and named outputs that hold it); never before, so no node is made twice in a file; and never past the file's end.
use super::super::fixtures::{RUN, every, every_node, first_column, node_of, plan, ran, request};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{Input as StepInput, StepId};
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
    };
    let intents = vec![RUN; plans.len()];
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
    watch.watch(&[functions]);
    let _ran = ran(&request, &intents, &[]);
    Ok(Cow::Owned(watch.taken().join("; ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
