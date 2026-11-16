//! What a file's sources are asked for, through the request's per-file route: the relations of the rules whose intent for the
//! file is Run, once each, and nothing for a rule that does not run on it. A rule that runs but has no candidate in the file
//! still reads its relations: its fault, its coverage and its rows are what they are with them.
use super::super::fixtures::{RUN, engine, every, plan, ran, request};
use ryo_execution_domain::domain::rule_intent::types::{
    ApplicabilityReason, CaptureIntent, DiagnosticDelivery, FileRuleIntent, ObservationDemand,
};
use ryo_relations::domain::relations::types::{Missing, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, JoinColumn, LiteralText, Operand,
    ParameterId, PlanComparison, QueryLiteral, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan, PlanStep};
use ryo_values::domain::stored_facts::types::{PartialAt, PartialCause, PartialScope, ReadInPart};
use std::borrow::Cow;

/// The request and the file a case runs.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Rule 0 selects the functions and runs; rule 1 selects the comments and its diagnostic does not apply to the file.
    SecondNotRun,
    /// The same two rules, neither running on the file.
    NoneRun,
    /// The same two rules, rule 1 run for its named outputs alone: its diagnostic is disabled on the file.
    SecondObservationsOnly,
    /// A rule rooted at classes, the file having none, that extends their rows with a parameter it binds to nothing.
    RootedExtension,
    /// A rule rooted at classes joined with the functions, which the file's source says it read in part.
    RootedPartial,
    /// A rule keeping the functions that no class of the file names, the file having no class.
    RootedAnti,
}

type Input = Scenario;
/// What the file's source was asked for, then the file's rules and end, as text.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_not_run_on_a_file_reads_nothing_for_it"),
        word: Word::Succeeds,
        input: || Scenario::SecondNotRun,
        expect: Expect::Value(Cow::Borrowed(
            "[[Function]] rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("a_file_with_no_running_rule_reads_nothing"),
        word: Word::Succeeds,
        input: || Scenario::NoneRun,
        expect: Expect::Value(Cow::Borrowed("[[]] finished; partial []")),
    },
    Case {
        name: CaseName("a_rule_run_for_its_named_outputs_alone_reads_its_relations"),
        word: Word::Succeeds,
        input: || Scenario::SecondObservationsOnly,
        expect: Expect::Value(Cow::Borrowed(
            "[[Function, Comment]] rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . rule 1: withheld; ran . finished; partial []",
        )),
    },
    Case {
        name: CaseName("a_rule_with_no_candidate_still_raises_its_extension_fault"),
        word: Word::Succeeds,
        input: || Scenario::RootedExtension,
        expect: Expect::Value(Cow::Borrowed(
            "[[]] rule 0: failed Unbound; unreached. unfinished; partial []",
        )),
    },
    Case {
        name: CaseName("a_rule_with_no_candidate_keeps_its_partial_coverage"),
        word: Word::Succeeds,
        input: || Scenario::RootedPartial,
        expect: Expect::Value(Cow::Borrowed(
            "[[Function]] rule 0: partial [] affecting [Some(FileId(0))]; ran . finished; partial [Partial { relation: Function, cause: PartialCause(\"the reader stopped\"), lacking: Any, at: File(FileId(0)) }]",
        )),
    },
    Case {
        name: CaseName("a_rule_with_no_candidate_still_emits_its_anti_rows"),
        word: Word::Succeeds,
        input: || Scenario::RootedAnti,
        expect: Expect::Value(Cow::Borrowed(
            "[[Function]] rule 0: complete [Node(NodeId(1)), Symbol(Symbol(0)), File(FileId(0)), Span(Span { start: Offset(0), end: Offset(4) })]; ran . finished; partial []",
        )),
    },
];

/// A diagnostic that does not apply to the file.
const NOT_RUN: FileRuleIntent =
    FileRuleIntent::Skipped(ApplicabilityReason::DiagnosticNotApplicable);

/// A rule run on the file for its named outputs alone, as core runs a rule that does not govern the file but whose outputs a
/// governing rule reads: its diagnostic disabled, its named outputs required, only its observation frontiers captured.
const OBSERVATIONS_ONLY: FileRuleIntent = FileRuleIntent::Run {
    diagnostic: DiagnosticDelivery::Disabled,
    observations: ObservationDemand::Required,
    capture: CaptureIntent::Observations,
};

/// A step in each file that selects the tree nodes of the classes: a select rooted at their kind.
fn classes() -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Select {
            from: StepInput::Relation(Relation::SyntaxTree),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("class_definition".into()))),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        },
    }
}

/// A step in each file doing `operation`.
fn per_file(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation,
    }
}

/// The request's plans and each rule's intent for the file, and what the file's source says it read in part.
fn scenario(input: Scenario) -> (Vec<Plan>, Vec<FileRuleIntent>, Vec<ReadInPart>) {
    let functions = || plan(vec![every(Relation::Function)], 0, Vec::new());
    let comments = || plan(vec![every(Relation::Comment)], 0, Vec::new());
    match input {
        Scenario::SecondNotRun => (
            vec![functions(), comments()],
            vec![RUN, NOT_RUN],
            Vec::new(),
        ),
        Scenario::NoneRun => (
            vec![functions(), comments()],
            vec![NOT_RUN, FileRuleIntent::Undemanded],
            Vec::new(),
        ),
        Scenario::SecondObservationsOnly => (
            vec![functions(), comments()],
            vec![RUN, OBSERVATIONS_ONLY],
            Vec::new(),
        ),
        Scenario::RootedExtension => (
            vec![plan(
                vec![
                    classes(),
                    per_file(Operation::Extend {
                        from: StepInput::Step(StepId(0)),
                        values: vec![Operand::Parameter(ParameterId(0))],
                    }),
                ],
                1,
                Vec::new(),
            )],
            vec![RUN],
            Vec::new(),
        ),
        Scenario::RootedPartial => (
            vec![plan(
                vec![
                    classes(),
                    every(Relation::Function),
                    per_file(Operation::Join {
                        left: StepInput::Step(StepId(0)),
                        right: StepInput::Step(StepId(1)),
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(0)],
                        output: vec![JoinColumn {
                            side: Side::Left,
                            column: ColumnIndex(0),
                        }],
                        filter: Vec::new(),
                        view: Vec::new(),
                        reach: None,
                    }),
                ],
                2,
                Vec::new(),
            )],
            vec![RUN],
            vec![ReadInPart {
                relation: Relation::Function,
                cause: PartialCause("the reader stopped".into()),
                lacking: Missing::Any,
                at: PartialScope::OneFile(PartialAt(0)),
            }],
        ),
        Scenario::RootedAnti => (
            vec![plan(
                vec![
                    every(Relation::Function),
                    classes(),
                    per_file(Operation::Antijoin {
                        left: StepInput::Step(StepId(0)),
                        right: StepInput::Step(StepId(1)),
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(0)],
                    }),
                ],
                2,
                Vec::new(),
            )],
            vec![RUN],
            Vec::new(),
        ),
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let (plans, intents, said) = scenario(input);
    let request = request(plans, engine());
    let ran = ran(&request, &intents, &said);
    Ok(Cow::Owned(format!("{:?} {}", ran.asked, ran.rules)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
