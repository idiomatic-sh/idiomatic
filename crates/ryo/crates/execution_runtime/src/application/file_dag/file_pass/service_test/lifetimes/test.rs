//! How long a node's rows live in a file, through the request's per-file route, seen at the operators port: a node's rows go
//! once its last consumer is done, whichever comes last of its node readers and its engine uses (the rules that emit it, the
//! frontiers and named outputs that hold it), a consumer that does not run on the file counting as done from the start; never
//! before, so no node is made twice in a file; and never past the file's end. Each case answers the file's own outcome too.
use super::super::fixtures::{RUN, every, every_node, first_column, node_of, plan, ran};
use super::fixtures::watched;
use ryo_execution_domain::domain::rule_intent::types::{ApplicabilityReason, FileRuleIntent};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, Operand, PlanComparison,
    RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_plans::types::Plan;
use std::borrow::Cow;
use std::sync::PoisonError;

/// How a rule does not run on the file.
#[derive(Clone, Copy, Debug)]
enum NotRun {
    /// Its diagnostic does not apply to the file.
    Skipped,
    /// It does not govern the file and no governing rule reads its outputs.
    Undemanded,
}

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
    /// Rules 0 and 1 emit the functions, rule 0 not running on the file, its turn ended before rule 1 made the node; rule 2
    /// emits the comments.
    UseBeforeMade(NotRun),
    /// Rule 0 emits the functions; rule 1, not running on the file, projects their first column; rule 2 emits the comments.
    ReaderNotRun(NotRun),
    /// Both rules read one select of the semantic edges. Rule 0, not running on the file, keeps the edges of no later sibling
    /// (an antijoin over a self join of the select, which the planner probes as an ordered probe, the self join and its
    /// select and projection no longer reading); rule 1 joins the select with the projection of its parents and emits each
    /// edge's parent.
    OrderedNotRun(NotRun),
}

type Input = Scenario;
/// What the operators saw, in order: each run with whether each watched node held rows before and after it, and each relation
/// lent; then, after `|`, the file's rules and end. For a stopped file: its rules and end, then what the next file's first run
/// saw, and the next file's rules and end.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_buffer_is_released_after_its_last_consumer"),
        word: Word::Succeeds,
        input: || Scenario::LastEngineUse,
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 0 before 0:held; ran 0 after 0:held; run 1 before 0:absent; lend Comment; ran 1 after 0:absent | rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran captured. rule 1: complete [Node(NodeId(2)), Symbol(Symbol(1)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) }), Flag(Lowered)]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("an_engine_use_before_a_later_node_reader_keeps_the_rows"),
        word: Word::Succeeds,
        input: || Scenario::EngineUseThenReader,
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 1 before 0:held; ran 1 after 0:absent | rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 1: complete [Node(NodeId(1))]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("a_node_read_then_captured_is_made_once"),
        word: Word::Succeeds,
        input: || Scenario::ReadThenCaptured,
        expect: Expect::Value(Cow::Borrowed(
            "run 1 before 0:absent; lend Function; ran 1 after 0:held; run 0 before 0:held; ran 0 after 0:held | rule 0: complete [Node(NodeId(1))]; ran captured. finished; partial []",
        )),
    },
    Case {
        name: CaseName("a_rule_not_run_that_shares_a_node_leaves_it_to_the_later_rule"),
        word: Word::Succeeds,
        input: || Scenario::SkippedSharesALastProbe,
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent 2:absent; lend Function; lend Comment; ran 0 after 0:held 2:absent; run 2 before 0:absent 2:absent; lend Function; lend Comment; ran 2 after 0:absent 2:held | rule 0: complete []; ran . rule 2: complete []; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("an_engine_use_of_a_skipped_rule_done_before_its_node_is_made_still_counts"),
        word: Word::Succeeds,
        input: || Scenario::UseBeforeMade(NotRun::Skipped),
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 1 before 0:absent; lend Comment; ran 1 after 0:absent | rule 1: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 2: complete [Node(NodeId(2)), Symbol(Symbol(1)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) }), Flag(Lowered)]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName(
            "an_engine_use_of_an_undemanded_rule_done_before_its_node_is_made_still_counts",
        ),
        word: Word::Succeeds,
        input: || Scenario::UseBeforeMade(NotRun::Undemanded),
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 1 before 0:absent; lend Comment; ran 1 after 0:absent | rule 1: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 2: complete [Node(NodeId(2)), Symbol(Symbol(1)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) }), Flag(Lowered)]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("a_reader_in_a_skipped_rule_holds_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::ReaderNotRun(NotRun::Skipped),
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 2 before 0:absent; lend Comment; ran 2 after 0:absent | rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 2: complete [Node(NodeId(2)), Symbol(Symbol(1)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) }), Flag(Lowered)]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("a_reader_in_an_undemanded_rule_holds_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::ReaderNotRun(NotRun::Undemanded),
        expect: Expect::Value(Cow::Borrowed(
            "run 0 before 0:absent; lend Function; ran 0 after 0:held; run 2 before 0:absent; lend Comment; ran 2 after 0:absent | rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 2: complete [Node(NodeId(2)), Symbol(Symbol(1)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) }), Flag(Lowered)]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("a_skipped_ordered_existence_leaves_its_select_to_the_running_rule"),
        word: Word::Succeeds,
        input: || Scenario::OrderedNotRun(NotRun::Skipped),
        expect: Expect::Value(Cow::Borrowed(
            "run 6 before 0:absent; lend SemanticChild; ran 6 after 0:absent | rule 1: complete [Node(NodeId(1))]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("an_undemanded_ordered_existence_leaves_its_select_to_the_running_rule"),
        word: Word::Succeeds,
        input: || Scenario::OrderedNotRun(NotRun::Undemanded),
        expect: Expect::Value(Cow::Borrowed(
            "run 6 before 0:absent; lend SemanticChild; ran 6 after 0:absent | rule 1: complete [Node(NodeId(1))]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("no_buffer_outlives_the_file"),
        word: Word::Succeeds,
        input: || Scenario::StoppedFile,
        expect: Expect::Value(Cow::Borrowed(
            "rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 1: failed WrongOperation; unreached. unfinished; partial [] next: run 0 before 0:absent 1:absent | rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 1: complete [Node(NodeId(2)), Symbol(Symbol(1)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) }), Flag(Lowered)]; ran . rule 2: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . finished; partial []",
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
        Scenario::UseBeforeMade(_) => vec![functions(), functions(), comments()],
        Scenario::OrderedNotRun(_) => vec![later_siblings_absent(), parents_joined()],
        Scenario::ReaderNotRun(_) => vec![
            functions(),
            plan(
                vec![every(Relation::Function), first_column(0)],
                1,
                Vec::new(),
            ),
            comments(),
        ],
        Scenario::SkippedSharesALastProbe => {
            let unremarked = |left: StepInput| PlanStep {
                written: Default::default(),
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
    let not_run = |how: NotRun| match how {
        NotRun::Skipped => FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable),
        NotRun::Undemanded => FileRuleIntent::Undemanded,
    };
    let skipped = match input {
        Scenario::SkippedSharesALastProbe => Some((
            1,
            FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable),
        )),
        Scenario::UseBeforeMade(how) => Some((0, not_run(how))),
        Scenario::ReaderNotRun(how) => Some((1, not_run(how))),
        Scenario::OrderedNotRun(how) => Some((0, not_run(how))),
        Scenario::LastEngineUse
        | Scenario::EngineUseThenReader
        | Scenario::ReadThenCaptured
        | Scenario::StoppedFile => None,
    };
    if let Some((rule, intent)) = skipped
        && let Some(slot) = intents.get_mut(rule)
    {
        *slot = intent;
    }
    let (request, watch) = watched(plans);
    let functions = node_of(&request, 0, 0);
    if let Scenario::StoppedFile = input {
        *watch.failing.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(node_of(&request, 1, 0));
        let stopped = ran(&request, &intents, &[]);
        *watch.failing.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let _stopped_file = watch.taken();
        watch.watch(&every_node(&request));
        let next = ran(&request, &intents, &[]);
        let first = watch.taken().into_iter().next();
        return Ok(Cow::Owned(format!(
            "{} next: {} | {}",
            stopped.rules,
            first.unwrap_or_default(),
            next.rules
        )));
    }
    if let Scenario::SkippedSharesALastProbe = input {
        // Both probes watched: the first, and the second, the shared index's last probe, which the skipped rule emits.
        watch.watch(&[functions, node_of(&request, 1, 1)]);
    } else {
        watch.watch(&[functions]);
    }
    let ran = ran(&request, &intents, &[]);
    Ok(Cow::Owned(format!(
        "{} | {}",
        watch.taken().join("; "),
        ran.rules
    )))
}

/// A step in each file doing `operation`.
fn per_file(operation: Operation) -> PlanStep {
    PlanStep {
        written: Default::default(),
        scope: StepScope::PerFile,
        operation,
    }
}

/// A column of a join's answer: `column` of the `side` input.
const fn column_of(side: Side, column: u32) -> JoinColumn {
    JoinColumn {
        side,
        column: ColumnIndex(column),
    }
}

/// Every semantic edge (parent, child, ordinal, field, file) with no later sibling: the antijoin of the edges' select with the
/// distinct (parent, child, ordinal) of the edges a sibling of the same parent follows, over a self join of the select on
/// the parent; the shape the planner probes as an ordered probe.
fn later_siblings_absent() -> Plan {
    let select = || StepInput::Step(StepId(0));
    let key = || vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)];
    plan(
        vec![
            every(Relation::SemanticChild),
            per_file(Operation::Join {
                left: select(),
                right: select(),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![
                    column_of(Side::Left, 0),
                    column_of(Side::Left, 1),
                    column_of(Side::Left, 2),
                    column_of(Side::Right, 2),
                ],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            }),
            per_file(Operation::Select {
                from: StepInput::Step(StepId(1)),
                comparisons: vec![PlanComparison {
                    test: Test::LessThan,
                    left: Operand::Column(ColumnIndex(2)),
                    right: Operand::Column(ColumnIndex(3)),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                kept: Vec::new(),
            }),
            per_file(Operation::Project {
                from: StepInput::Step(StepId(2)),
                columns: key(),
                distinct: RepeatedRows::Dropped,
            }),
            per_file(Operation::Antijoin {
                left: select(),
                right: StepInput::Step(StepId(3)),
                left_on: key(),
                right_on: key(),
            }),
        ],
        4,
        Vec::new(),
    )
}

/// Each semantic edge's parent: the edges' select joined on the parent with the projection of its parents, repeats kept.
fn parents_joined() -> Plan {
    plan(
        vec![
            every(Relation::SemanticChild),
            first_column(0),
            per_file(Operation::Join {
                left: StepInput::Step(StepId(0)),
                right: StepInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![column_of(Side::Left, 0)],
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            }),
        ],
        2,
        Vec::new(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
